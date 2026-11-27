//! Mission, space combat, bombardment and assault results applied to the
//! game and logged, moved out of `main.rs`. The code reads `main.rs`'s
//! items through `super`.

use super::*;

pub(super) fn apply_mission_result(
    result: &rebellion_core::missions::MissionResult,
    continuing: bool,
    revolting: bool,
    world: &mut GameWorld,
    research_state: &mut ResearchState,
    log: &mut MessageLog,
    message_texts: &std::collections::HashMap<u16, String>,
    #[cfg(not(target_arch = "wasm32"))] audio: Option<(&mut audio::AudioEngine, &AudioVolumeState)>,
) {
    let faction_name = match result.faction {
        MissionFaction::Alliance => "Alliance",
        MissionFaction::Empire => "Empire",
    };
    let kind_name = match result.kind {
        MissionKind::Diplomacy => "Diplomacy",
        MissionKind::Recruitment => "Recruitment",
        MissionKind::Sabotage => "Sabotage",
        MissionKind::Assassination => "Assassination",
        MissionKind::Espionage => "Espionage",
        MissionKind::Rescue => "Rescue",
        MissionKind::Abduction => "Abduction",
        MissionKind::InciteUprising => "Incite Uprising",
        MissionKind::SubdueUprising => "Subdue Uprising",
        MissionKind::DeathStarSabotage => "Death Star Sabotage",
        MissionKind::Reconnaissance => "Reconnaissance",
        MissionKind::ShipDesignResearch => "Ship Design Research",
        MissionKind::FacilityDesignResearch => "Facility Design Research",
        MissionKind::TroopTrainingResearch => "Troop Training Research",
        MissionKind::Autoscrap => "Autoscrap",
        MissionKind::Move => "Move",
        MissionKind::Return => "Return",
        MissionKind::Autorouting => "Autorouting",
        MissionKind::Adrift => "Adrift",
        MissionKind::JediTraining => "Jedi Training",
        MissionKind::Dagobah => "Dagobah",
        MissionKind::Bounty => "Bounty",
        MissionKind::Palace => "Palace",
        MissionKind::Pickup => "Pickup",
    };
    let sys_name = world
        .systems
        .get(result.target_system)
        .map_or_else(|| "unknown".into(), |s| s.name.clone());
    let outcome_str = match result.outcome {
        rebellion_core::missions::MissionOutcome::Success => "succeeded",
        rebellion_core::missions::MissionOutcome::Failure => "failed",
    };

    let category = match result.kind {
        MissionKind::Diplomacy | MissionKind::InciteUprising | MissionKind::SubdueUprising => {
            MessageCategory::Diplomacy
        }
        MissionKind::Recruitment
        | MissionKind::Sabotage
        | MissionKind::Assassination
        | MissionKind::Espionage
        | MissionKind::Rescue
        | MissionKind::Abduction
        | MissionKind::DeathStarSabotage
        | MissionKind::Reconnaissance
        | MissionKind::ShipDesignResearch
        | MissionKind::FacilityDesignResearch
        | MissionKind::TroopTrainingResearch
        | MissionKind::Autoscrap
        | MissionKind::Move
        | MissionKind::Return
        | MissionKind::Autorouting
        | MissionKind::Adrift
        | MissionKind::JediTraining
        | MissionKind::Dagobah
        | MissionKind::Bounty
        | MissionKind::Palace
        | MissionKind::Pickup => MessageCategory::Mission,
    };
    // FUN_004927c0: the mission type's picture and the leader's portrait.
    let reported = {
        use pictures::ReportedMission;
        match result.kind {
            MissionKind::Diplomacy => ReportedMission::Diplomacy,
            MissionKind::Espionage => ReportedMission::Espionage,
            MissionKind::InciteUprising => ReportedMission::InciteUprising,
            MissionKind::SubdueUprising => ReportedMission::SubdueUprising,
            // FUN_004927c0 case 0x53: picture 0x3f9, sound 0x47f.
            MissionKind::ShipDesignResearch
            | MissionKind::FacilityDesignResearch
            | MissionKind::TroopTrainingResearch => ReportedMission::Research,
            _ => ReportedMission::Other,
        }
    };
    // FUN_004927c0's +0x30 by family and +0x32, the leader's verdict.
    let sounds = {
        let leader = result.character.and_then(|key| world.characters.get(key));
        MessageSounds::mission_report(
            reported,
            result.outcome == rebellion_core::missions::MissionOutcome::Success,
            result.faction == MissionFaction::Alliance,
            leader.map_or(0, |c| c.dat_id.index()),
            leader.is_some_and(|c| c.dagobah_completed),
        )
    };
    // A Recon led by a special force shows the force's own (case 0x54).
    let force_picture = result
        .lead_force
        .filter(|_| result.kind == MissionKind::Reconnaissance)
        .and_then(|force| world.special_forces.get(force))
        .and_then(|force| pictures::special_force_report(force.class_dat_id));
    let picture = force_picture.unwrap_or_else(|| {
        let mut picture = pictures::mission_report(
            reported,
            result.outcome == rebellion_core::missions::MissionOutcome::Success,
            pictures::reader(result.faction == MissionFaction::Alliance),
            result
                .character
                .and_then(|character| pictures::portrait_key(world, character)),
        );
        // The leader's own picture (FUN_004c5000): Luke after Dagobah is
        // the Jedi Knight's (A0 2026-11-16-edited-saves/x-59).
        if let Some(own) = result
            .character
            .and_then(|character| pictures::report_leader_portrait(world, character))
        {
            picture.foreground = Some(own);
        }
        picture
    });
    // The report's names, read before the effects.
    let (leader, target, recruit, target_killed) = report_names(world, result);
    let returning = result
        .returning_to
        .and_then(|place| return_place_name(world, place));
    // SFX + voice lines for mission outcomes
    #[cfg(not(target_arch = "wasm32"))]
    if let Some((audio_engine, audio_vol)) = audio {
        if result.outcome == rebellion_core::missions::MissionOutcome::Success {
            audio_engine.play_sfx(SfxKind::MissionSuccess, audio_vol);
        } else {
            audio_engine.play_sfx(SfxKind::MissionFail, audio_vol);
        }
    }

    // A research level reached by this result (FUN_004927c0 reads the
    // mission's +0xac as "valuable results").
    let mut valuable = false;
    for effect in &result.effects {
        match effect {
            MissionEffect::SupportGained {
                system,
                side,
                points,
            } => rebellion_core::uprising::apply_support_change(world, *system, *side, *points),
            MissionEffect::SkillRaised {
                character,
                skill,
                amount,
            } => rebellion_core::missions::raise_skill(world, *character, *skill, *amount),
            MissionEffect::ResearchPoints {
                faction,
                tech,
                points,
            } => {
                let is_alliance = *faction == MissionFaction::Alliance;
                let was_done = research_state.progress(is_alliance, *tech).done;
                for level in research_state.add_points(world, is_alliance, *tech, *points) {
                    valuable = true;
                    // The same class as the economy's (kind 4): Display
                    // opens the class's Encyclopedia entry here too.
                    if let Some(message) = research_complete_message(
                        world,
                        is_alliance,
                        *tech,
                        level,
                        result.tick,
                        message_texts,
                    ) {
                        log.push(message);
                    }
                }
                // Notifications 0x129..0x12b, Research Exhausted
                // (FUN_0048abc0): the tree was just marked Done.
                if !was_done && research_state.progress(is_alliance, *tech).done {
                    if let Some((title, text)) =
                        rebellion_render::story_messages::research_exhausted_texts(*tech, message_texts)
                    {
                        log.push(filed(
                            GameMessage::new(result.tick, text, MessageCategory::Event)
                                .with_title(title)
                                .with_picture_if(Some(pictures::research()))
                                .with_sounds(MessageSounds::research())
                                // FUN_0048abc0 (kind 8) writes code 4.
                                .with_advisor_event(4)
                                .plain(),
                            MessageRail::Manufacturing,
                            Some(RailAudience::side(is_alliance)),
                        ));
                    }
                }
            }
            MissionEffect::UprisingIncident(event) => {
                rebellion_core::uprising::apply_uprising_event(world, event);
            }
            MissionEffect::SystemIntelligenceGathered { system, faction } => {
                // Event319_Handle explores the side's copy only
                // (object_views::side_explored, ghidra/notes/fog-of-war.md);
                // Event319_Handle (FUN_0050d5a0): the side's copies of the
                // system's objects take their state.
                world
                    .object_views
                    .queue_push((*faction).into(), *system);
            }
            MissionEffect::CharacterRecruited {
                character,
                system,
                faction,
                pool_emptied,
            } => {
                // FUN_0055fc80 -> FUN_0052f590: the side's RecruitmentDone
                // turning on posts 0x12c, Recruitment Done (FUN_0048ae40).
                let side: Faction = (*faction).into();
                let done = *pool_emptied && !world.recruit_pool_empty(side);
                rebellion_core::missions::recruit_character(
                    world,
                    *character,
                    *system,
                    *faction,
                    *pool_emptied,
                );
                if done {
                    if let Some(message) = rebellion_render::story_messages::recruitment_done_message(
                        side == Faction::Alliance,
                        result.tick,
                        message_texts,
                    ) {
                        log.push(message);
                    }
                }
            }
            MissionEffect::TargetSabotaged { target, system } => {
                let victim = rebellion_core::missions::sabotage_victim(world, *target);
                rebellion_core::missions::destroy_target(world, *target);
                if !matches!(
                    target,
                    rebellion_core::missions::MissionTarget::DeathStar(_)
                ) {
                    if let Some(message) =
                        saboteurs_strike_message(world, victim, *system, result.tick, message_texts)
                    {
                        log.push(message);
                    }
                } else {
                    // Notification 0x23, Death Star Sabotaged
                    // (FUN_0048d470): a story movie only (kind 6, queued
                    // with the effect), no Message Index row. port: the
                    // line stays in the log, off the index.
                    log.push(GameMessage::new(
                        result.tick,
                        "The Death Star has been sabotaged.".to_string(),
                        MessageCategory::Mission,
                    ));
                }
            }
            // As the integrator: off every roster and marked killed, kept
            // in the arena so its messages can still name it.
            MissionEffect::CharacterKilled { character, .. } => {
                rebellion_core::missions::kill_character(world, *character);
            }
            MissionEffect::CharacterCaptured {
                character,
                captured_by,
                ..
            } => {
                // Set captivity state — do NOT flip is_alliance/is_empire.
                // Those fields encode faction *identity*, not current holder.
                // Flipping them corrupts escape direction (check_escapes uses
                // is_alliance to determine where the character escapes TO).
                if let Some(c) = world.characters.get_mut(*character) {
                    c.is_captive = true;
                    c.captured_by = Some(match captured_by {
                        MissionFaction::Alliance => Faction::Alliance,
                        MissionFaction::Empire => Faction::Empire,
                    });
                    c.capture_tick = Some(result.tick);
                }
                // Remove from current fleet assignments
                for (_, fleet) in &mut world.fleets {
                    fleet.characters.retain(|&k| k != *character);
                }
            }
            MissionEffect::SpecialForceDestroyed { unit } => {
                rebellion_core::missions::destroy_special_force(world, *unit);
            }
            MissionEffect::CharacterInjured { character, injury } => {
                // Slot +0x2f0 (FUN_004ef6d0) adds the injury to +0x94.
                if let Some(c) = world.characters.get_mut(*character) {
                    c.injure(*injury);
                }
                if *injury > 0 {
                    push_character_health(log, world, *character, result.tick, message_texts);
                }
            }
            // The detection's Force gains come with the advance
            // (`apply_advance_effects`); a result applies one the same way.
            MissionEffect::ForceExperienceGained { character, amount } => {
                let old = force_level(world, *character);
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*amount);
                }
                push_force_growth(log, world, *character, old, result.tick, message_texts);
            }
            // The advance's effects (`apply_advance_effects`).
            MissionEffect::MemberEnteredAbode { member, abode } => {
                rebellion_core::missions::enter_abode(world, *member, *abode);
            }
            MissionEffect::DagobahCompleted {
                character,
                force_gain,
            } => {
                let old = force_level(world, *character);
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*force_gain);
                    c.dagobah_completed = true;
                }
                // Under the Jedi Knight's portrait, as A0
                // alliance-game2/luke-force-growth (day 643) shows it.
                push_force_growth(log, world, *character, old, result.tick, message_texts);
            }
            // FUN_005712b0 -> FUN_004ee1d0: a Jedi Training gain. The new
            // Force posts 0x1e1 before the mission's report (A0
            // 2026-11-16-edited-saves/x-58 lists "Jan Dodonna Force Growth"
            // under "Luke Skywalker Mission Report").
            MissionEffect::ForceTrainingGained { character, amount } => {
                let old = force_level(world, *character);
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_training(*amount);
                }
                push_force_growth(log, world, *character, old, result.tick, message_texts);
            }
            // The encounter's keys and Luke's heritage, set by the mission
            // detection (`missions::apply_advance_effects`).
            MissionEffect::EncounterBegan { .. }
            | MissionEffect::EncounterEnded { .. }
            | MissionEffect::HeritageLearned { .. }
            | MissionEffect::CapturedByBountyHunters { .. }
            | MissionEffect::EscapeBarred { .. }
            | MissionEffect::MemberOnMission { .. }
            | MissionEffect::MemberBoarded { .. }
            | MissionEffect::MemberSeated { .. }
            | MissionEffect::AdriftSettled { .. }
            | MissionEffect::DiedOfInjury { .. } => {
                rebellion_core::missions::apply_advance_effects(world, std::slice::from_ref(effect));
            }
            // Notifications 0x212 and 0x231: their messages are posted
            // with the result; a pickup's prisoners leave as the mission
            // ends.
            MissionEffect::BountyAttack { .. }
            | MissionEffect::PickupInProgress
            | MissionEffect::PrisonerPickedUp { .. }
            | MissionEffect::TraitorIdentified { .. } => {}
            MissionEffect::CharacterRescued {
                character,
                returned_to,
                ..
            } => {
                // Restore character to the specified faction
                if let Some(c) = world.characters.get_mut(*character) {
                    match returned_to {
                        MissionFaction::Alliance => {
                            c.is_alliance = true;
                            c.is_empire = false;
                        }
                        MissionFaction::Empire => {
                            c.is_alliance = false;
                            c.is_empire = true;
                        }
                    }
                    c.is_captive = false;
                    c.captured_by = None;
                    c.capture_tick = None;
                }
            }
            MissionEffect::MemberAvailable { member } => {
                rebellion_core::missions::set_on_mission(world, *member, false);
            }
            MissionEffect::MemberMoved { member, to } => {
                rebellion_core::missions::move_member(world, *member, *to);
            }
            MissionEffect::CharacterEscaped {
                character,
                escaped_to_alliance,
            } => {
                escape_character(
                    world,
                    *character,
                    *escaped_to_alliance,
                    result.tick,
                    log,
                    message_texts,
                );
            }
            MissionEffect::UprisingSubdued {
                system,
                side,
                support_gain,
            } => {
                // FUN_00569c20 -> FUN_0050c9f0. The caller then runs the
                // FUN_0050c910 end check via apply_mission_state_effects.
                rebellion_core::uprising::apply_support_change(
                    world,
                    *system,
                    *side,
                    *support_gain,
                );
            }
        }
    }
    // Notifications 0x16 and 0x17, Mission Report and Mission Failed.
    // FUN_004927c0: Diplomacy, research and Subdue Uprising end on TEXTSTRA
    // 0x70db and ask (kind 5).
    let asks =
        rebellion_render::story_messages::mission_report_asks_to_continue(result.kind, continuing);
    // FUN_004927c0's TEXTSTRA report, read after the effects as the
    // notification arrives after them (Diplomacy names the new support
    // side); the port's own line for the kinds not yet ported.
    let support = world.systems.get(result.target_system).and_then(|system| {
        rebellion_core::economy::support_side(system, &world.gnprtb, world.difficulty_index)
    });
    let shipped = rebellion_render::story_messages::mission_report_texts(
        result.kind,
        &rebellion_render::story_messages::ReportFacts {
            success: result.outcome == rebellion_core::missions::MissionOutcome::Success,
            leader: leader.as_ref().map(|(name, major)| (name.as_str(), *major)),
            system: &sys_name,
            support,
            target: target.as_deref(),
            target_killed,
            recruit: recruit.as_deref(),
            revolting,
            valuable,
            returning: returning.as_deref(),
        },
        message_texts,
    );
    let question = message_texts
        .get(&0x70db)
        .filter(|_| asks)
        .map(|question| rebellion_render::story_messages::fill_message_text(question, &[]));
    let (title, text) = report_title_and_text(
        shipped,
        format!("{faction_name} {kind_name} mission at {sys_name} {outcome_str}"),
        question.as_deref(),
    );
    let mut report = GameMessage::at_system(result.tick, text, category, result.target_system)
        .with_picture(picture)
        .with_sounds(sounds)

        .with_mission(result.mission_id);
    if let Some(title) = title {
        report = report.with_title(title);
    }
    if asks {
        report = report.with_continue_question(result.mission_id);
    }
    log.push(filed(
        report
            // FUN_004927c0's speaker map on the report's leader (+0x74).
            .with_advisor_event(rebellion_render::story_messages::speaker_event(
                world,
                result.character,
            )),
        MessageRail::Mission,
        Some(RailAudience::side(
            result.faction == MissionFaction::Alliance,
        )),
    ));
}

// ---------------------------------------------------------------------------
// Knesset Shamash-Bet Dabora 2 (#F7): the old `apply_event_actions` duplicate
// was deleted here. The canonical implementation lives in
// `rebellion_data::integrator::apply_event_action_to_world` (pub #[inline]).
// `DisplayMessage` now routes through `GameEffect::StoryMessageDisplayed`
// and is drained into `msg_log` at the interactive tick call site.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Combat effect application helpers
// ---------------------------------------------------------------------------

/// Delegate to integrator's shared implementation.
/// The side that won a battle of `attacker_is_alliance`'s fleet against
/// another side's (`FUN_0050a040`), or none for a draw.
pub(super) fn battle_winner(
    winner: CombatSide,
    attacker_is_alliance: Option<bool>,
) -> Option<Faction> {
    let attacker = attacker_is_alliance?;
    match winner {
        CombatSide::Attacker => Some(Faction::of_alliance(attacker)),
        CombatSide::Defender => Some(Faction::of_alliance(!attacker)),
        CombatSide::Draw => None,
    }
}

pub(super) fn apply_space_combat_result(
    result: &rebellion_core::combat::SpaceCombatResult,
    world: &mut GameWorld,
    combat_rolls: &[f64],
) {
    // The abstract battle's seed takes the first roll; the lost ships'
    // passengers draw their injuries from the rest.
    rebellion_data::integrator::apply_space_combat_result_inner(
        result,
        world,
        combat_rolls.get(1..).unwrap_or_default(),
    );
}

/// The messages a bombardment leaves: what it destroyed, and the Rebel
/// HQ's destruction (notification `0x1f`).
pub(super) fn log_bombardment(
    world: &GameWorld,
    system: SystemKey,
    outcome: &rebellion_core::bombardment::BombardmentOutcome,
    tick: u64,
    log: &mut MessageLog,
) {
    let name = world
        .systems
        .get(system)
        .map_or("Unknown", |value| value.name.as_str());
    log.push(GameMessage::at_system(
        tick,
        format!(
            "Orbital bombardment at {name}: {} shot(s), {} target(s) destroyed",
            outcome.shots,
            outcome.destroyed.len()
        ),
        MessageCategory::Combat,
        system,
    ));
    if outcome
        .destroyed
        .contains(&rebellion_core::bombardment::Target::Headquarters)
    {
        // Notification 0x128 bit 0, Alliance Headquarters Destroyed
        // (FUN_0048e5a0): a story movie only (kind 6, from the victory
        // word), no Message Index row. port: the line stays in the log.
        log.push(GameMessage::new(
            tick,
            format!("Alliance headquarters destroyed at {name}"),
            MessageCategory::Combat,
        ));
    }
}

/// The Assault Summary (`ghidra/notes/planetary-assault.md`): each side's
/// losses and the regiments that landed.
pub(super) fn log_assault(
    world: &GameWorld,
    system: SystemKey,
    outcome: &rebellion_core::assault::AssaultOutcome,
    tick: u64,
    log: &mut MessageLog,
) {
    let name = world
        .systems
        .get(system)
        .map_or("Unknown", |value| value.name.as_str());
    log.push(GameMessage::at_system(
        tick,
        format!(
            "Planetary assault at {name}: {} of {} attacking and {} of {} defending regiment(s) lost, {} landed",
            outcome.lost_attackers.len(),
            outcome.attackers,
            outcome.lost_defenders.len(),
            outcome.defenders,
            outcome.landed.len()
        ),
        MessageCategory::Combat,
        system,
    ));
}

/// The Assault Summary reports of assaults and bombardments
/// (`ghidra/notes/assault-summary-window.md`):
/// the attacker's side sees the window at once (kind 7, `FUN_00497530`);
/// the other side gets a battle report on the Conflict rail (kind 3,
/// category `0x100`, `FUN_0048bc60`) that opens it.
pub(super) fn present_assault_reports(
    reports: Vec<rebellion_core::assault_report::AssaultReport>,
    world: &GameWorld,
    player_faction: MissionFaction,
    message_texts: &std::collections::HashMap<u16, String>,
    summaries: &mut rebellion_render::assault_summary::AssaultSummaryState,
    log: &mut MessageLog,
    tick: u64,
) {
    let alliance = player_faction == MissionFaction::Alliance;
    for report in reports {
        let (title, body) =
            rebellion_render::assault_summary::summary_texts(&report, world, message_texts);
        let picture = rebellion_render::message_log::MessagePicture {
            background: report.picture(),
            foreground: None,
        };
        let message = GameMessage::at_system(tick, body, MessageCategory::Combat, report.system)
            .with_title(title)
            .with_picture(picture)
            .on_rail(MessageRail::Conflict, RailAudience::side(alliance))
            .with_assault_report(report.clone());
        if report.opens_at_once(Faction::of_alliance(alliance)) {
            // FUN_0048a060 kind 7: FUN_005f5440 files the report, then
            // FUN_0041d7d0 opens it. The rail mask (+0x50), its light
            // (FUN_0041db20) and the rebuild (FUN_0048a2a0) are kind 3's,
            // so it lights nothing. A0 2026-11-14-bombard-assault/41..43.
            // Its class's kind (vtable +0x10, FUN_00497530) is 7, neither 3
            // nor 5 nor 4, so FUN_004697b0 leaves the index's Display grey
            // (A0 2026-11-17-edited-saves-5/i-05, the Empire's own
            // bombardment of Ghorman chosen).
            let id = log.push(GameMessage {
                unread: false,
                ..message.plain()
            });
            summaries.open(world, report, message_texts, Some(id));
            continue;
        }
        log.push(message);
    }
}

/// A planet destroyed by a Death Star (the Fleet menu's superlaser, or a
/// Destroy System order's `FUN_0058d990`): its units and facilities go, its
/// fleets stay, the characters standing there go adrift and `killed` die
/// (`death_star::cleanup_destroyed_system`).
#[expect(
    clippy::too_many_arguments,
    reason = "The planet's cleanup touches each of these states."
)]
/// A destroyed system's messages: each character it killed (the cleanup's
/// effects, drained into the log at once as the headless flow does through
/// the `PerceptionIntegrator`) and the destruction's story movie.
pub(super) fn destroyed_planet_messages(
    world: &GameWorld,
    name: &str,
    holder: Option<Faction>,
    effects: Vec<rebellion_core::effects::GameEffect>,
    msg_log: &mut MessageLog,
    tick: u64,
    message_texts: &std::collections::HashMap<u16, String>,
) {
    for effect in effects {
        if let rebellion_core::effects::GameEffect::CharacterKilled { character } = effect {
            // Notification 0x106, Character Health (FUN_0048fdf0).
            push_character_health(msg_log, world, character, tick, message_texts);
        }
    }
    // Notification 0x182, System Destroyed by the Death Star
    // (FUN_004950d0): a story movie, MDATA.101, whichever side plays.
    msg_log.queue_story_movie(
        rebellion_render::story_messages::death_star_destroyed_movie(name, holder, message_texts),
    );
}
