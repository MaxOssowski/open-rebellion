//! The advisor's schedule and advice, the briefing's actions, mission
//! aborts, the Message Index's actions and the agent's commands, moved out
//! of `main.rs`. The code reads `main.rs`'s items through `super`.

use super::*;

/// A new game's advisor schedule for `side`, from its GDATA messages
/// (`FUN_004c27f0`, `FUN_004c0710`); `None` when they cannot be read.
/// A loaded game's advisor from the agent's saved state, or none when the
/// save holds no schedule (FUN_00439550).
pub(super) fn loaded_advisor_schedule(
    gdata: &Path,
    side: AdvisorFaction,
    agent: &PlayerAgent,
) -> Option<rebellion_render::advisor_schedule::AdvisorSchedule> {
    let (last_day, enabled) = agent.advisor_state();
    if last_day.is_empty() {
        return None;
    }
    let messages = advisor_messages(gdata, side)?;
    macroquad::logging::info!(
        "[advisor] schedule restored messages={} due={} enabled={} translate={}",
        last_day.len(),
        last_day.iter().filter(|&&day| day != 0).count(),
        enabled,
        agent.translate()
    );
    Some(rebellion_render::advisor_schedule::AdvisorSchedule::from_save(
        side, messages, last_day, enabled,
    ))
}

pub(super) fn new_advisor_schedule(
    gdata: &Path,
    side: AdvisorFaction,
) -> Option<rebellion_render::advisor_schedule::AdvisorSchedule> {
    advisor_messages(gdata, side)
        .map(|messages| rebellion_render::advisor_schedule::AdvisorSchedule::new_game(side, messages))
}

/// The side's advisor messages (GDATA C3POACT.SPT / IMP22ACT.SPT).
pub(super) fn advisor_messages(
    gdata: &Path,
    side: AdvisorFaction,
) -> Option<
    std::collections::HashMap<u32, Vec<rebellion_render::advisor_schedule::ScriptEntry>>,
> {
    use rebellion_render::advisor_schedule::ScriptEntry;
    let script =
        match rebellion_data::load_advisor_script(gdata, side == AdvisorFaction::Alliance) {
            Ok(script) => script,
            Err(error) => {
                macroquad::logging::warn!("[advisor] messages unavailable error={:#}", error);
                return None;
            }
        };
    let messages = script
        .messages
        .into_iter()
        .map(|message| {
            let entries = message
                .entries
                .iter()
                .map(|entry| ScriptEntry {
                    record: entry.record,
                    flags: entry.flags,
                    a: entry.a,
                    b: entry.b,
                })
                .collect();
            (message.id, entries)
        })
        .collect();
    Some(messages)
}

/// The advisor's time, `DAT_006b28cc` (`FUN_004fcee0`): the scheduler's
/// sub-tick, plus one. hyp: the count starts at 1, as the briefing plays
/// on a new game's opening, which `FUN_004c0a60`'s next-free test (above
/// 0) needs.
pub(super) fn advisor_now(clock: &GameClock) -> u32 {
    u32::try_from(clock.sub_tick())
        .unwrap_or(u32::MAX)
        .saturating_add(1)
}

/// Post the agent's advice, each as its message (`FUN_0048a590` →
/// `FUN_0048b2e0`).
pub(super) fn post_agent_advice(
    posted: &[rebellion_core::agent_advice::AdviceEntry],
    alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
    msg_log: &mut MessageLog,
    reason: &str,
) {
    for entry in posted {
        let message =
            rebellion_render::story_messages::agent_advice_message(entry.number, alliance, tick, texts);
        macroquad::logging::info!(
            "[agent] advice posted number={} kind={} reason={} titled={}",
            entry.number,
            entry.kind,
            reason,
            message.is_some()
        );
        if let Some(message) = message {
            msg_log.push(message);
        }
    }
}

/// A briefing action on the galaxy view (`FUN_004c0fc0` / `FUN_004c30c0`):
/// a display, or a marked system with its caption; returns the marked
/// system's id. A character whose system is gone shows the display and
/// caption with no system marked (hyp: `FUN_005f5540` clears the view's
/// system before each case); without the Alliance headquarters the
/// Alliance's code 3 shows Display Off (`FUN_004c30c0` case 3).
pub(super) fn apply_briefing_action(
    action: rebellion_render::advisor_schedule::BriefingAction,
    world: &GameWorld,
    cockpit: &mut CockpitState,
    map: &mut GalaxyMapState,
    caption: &mut Option<u16>,
) -> Option<u32> {
    use rebellion_render::advisor_schedule::{
        ALLIANCE_HEADQUARTERS, BriefingAction, MarkTarget,
    };
    match action {
        BriefingAction::Display(mode) => {
            cockpit.gid_mode = mode;
            *caption = None;
            None
        }
        BriefingAction::Mark {
            target,
            mode,
            caption: text,
        } => {
            let system = match target {
                MarkTarget::System(index) => world
                    .systems
                    .iter()
                    .find(|(_, system)| system.dat_id.index() == index)
                    .map(|(key, _)| key),
                MarkTarget::ObjectSystem(ALLIANCE_HEADQUARTERS) => world
                    .systems
                    .iter()
                    .find(|(_, system)| {
                        system.is_headquarters
                            && system.control.is_controlled_by(Faction::Alliance)
                    })
                    .map(|(key, _)| key),
                // port: a major character's DatId holds MJCHARSD's id
                // without the family byte that REBEXE's object ids carry
                // (0x30000240 is family 0x30, id 0x240).
                MarkTarget::ObjectSystem(record) => world
                    .characters
                    .values()
                    .find(|character| {
                        character.is_major && character.dat_id.raw() == record & 0x00FF_FFFF
                    })
                    .and_then(|character| {
                        character.current_system.or_else(|| {
                            character
                                .current_fleet
                                .and_then(|fleet| world.fleets.get(fleet))
                                .map(|fleet| fleet.location)
                        })
                    }),
            };
            if system.is_none() && target == MarkTarget::ObjectSystem(ALLIANCE_HEADQUARTERS) {
                cockpit.gid_mode = rebellion_render::GidMode::DisplayOff;
                *caption = None;
                return None;
            }
            map.selected_system = system;
            cockpit.gid_mode = mode;
            *caption = Some(text);
            system
                .and_then(|key| world.systems.get(key))
                .map(|system| system.dat_id.raw())
        }
    }
}

/// The team of a mission icon's Abort (`0x250`): the side's missions it
/// names; empty for any other selection. The menu offers Abort when the
/// team is not empty (the order's `+0x18`, `FUN_0051fe20`: `FUN_00553770`
/// refuses an empty team with `0x16`), whatever the missions' phases. The
/// phase is checked only when the order is given (`+0x1c`, `FUN_0051ff30`
/// -> `FUN_00553960` -> event `0x261` `+0x1c` -> the mission's `+0x1c4` ->
/// `FUN_00521600`). A0 personnel-states/37: Mon Calamari's research
/// missions offer Abort, which then asks nothing; Sullust's empty team
/// greys it.
pub(super) fn abort_missions(
    world: &GameWorld,
    fog: &rebellion_core::fog::FogState,
    missions: &MissionState,
    selection: Option<MenuObject>,
    faction: MissionFaction,
) -> Vec<u64> {
    // A Missions window's mission: that one alone (FUN_004a0400).
    if let Some(MenuObject::Mission(id)) = selection {
        return if missions
            .missions()
            .iter()
            .any(|mission| mission.id == id && mission.faction == faction)
        {
            vec![id]
        } else {
            Vec::new()
        };
    }
    let Some(MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Missions,
    }) = selection
    else {
        return Vec::new();
    };
    rebellion_render::quadrant_icons::own_visible_missions(
        world,
        fog,
        missions,
        Faction::from(faction),
        system,
    )
}

/// Carry out an Agent menu command or its accelerator: Manage Garrisons and
/// Manage Production toggle their module (`FUN_00439d60`), Agent Advice its
/// bit (`FUN_00439e80`).
/// Where a Finder's choice opens through `FUN_00429440`.
#[derive(Debug, Clone, Copy)]
pub(super) enum FinderTarget {
    /// A system alone: its sector window.
    Sector,
    /// The System Defenses window (kind 10).
    Defenses,
    /// The System Defenses window on its regiment page: the Troop Finder's
    /// Display (A0 personnel-states/15, 17, 18: Mon Calamari, Pzob and
    /// Umgul open on "Trooper Regiments").
    Regiments,
    /// The Fleet window (kind 4) with the fleet selected.
    Fleet(rebellion_core::ids::FleetKey),
    /// The Fleet window (kind 4) with a copied fleet selected.
    CopiedFleet(rebellion_render::fleet_window::FleetWindowEntry),
    /// The Missions window (kind 11) showing the member's mission.
    Missions(rebellion_core::missions::MissionMember),
    /// The Missions window (kind 11) choosing nothing.
    MissionsWindow,
}

/// A briefing action's effect on a new game's held galaxy: action 0xb
/// (kind 2) runs `FUN_0041dbe0` -> `FUN_0041e320`, which clears the game's
/// hold (`+0x14`); a new game then runs at the rate its setup chose, Slow
/// (`Class_0065ad70_vt10`: `FUN_00487eb0(2)`). Other actions, and a 0xb
/// with no new game held, leave the clock alone. True when it released.
pub(super) fn release_new_game_hold(kind: u32, code: u32, held: &mut bool, clock: &mut GameClock) -> bool {
    if kind != 2 || code != 0xb || !std::mem::take(held) {
        return false;
    }
    rebellion_render::game_speed::choose_game_speed(clock, GameSpeed::Slow);
    true
}

#[cfg(test)]
mod new_game_hold_tests {
    use super::*;

    #[test]
    fn the_briefings_action_0xb_runs_a_held_new_game_at_slow_once() {
        // User check on the original (Windows VM, 2026-11-24): at Expert,
        // with no advice window, day 1 starts at Slow after the briefing.
        let mut clock = GameClock::new();
        let mut held = true;
        assert!(!release_new_game_hold(2, 0xd, &mut held, &mut clock));
        assert!(!release_new_game_hold(1, 0xb, &mut held, &mut clock));
        assert_eq!(clock.speed, GameSpeed::Paused);
        assert!(release_new_game_hold(2, 0xb, &mut held, &mut clock));
        assert_eq!(clock.speed, GameSpeed::Slow);
        assert!(!held);
        clock.set_speed(GameSpeed::Fast);
        assert!(!release_new_game_hold(2, 0xb, &mut held, &mut clock), "a later 0xb keeps the player's speed");
        assert_eq!(clock.speed, GameSpeed::Fast);
    }
}

/// Carry out what the Message Index asks: the Advice speed hold
/// (`FUN_00487ff0`), marking read (`FUN_0048a530`, `FUN_00469de0`) and
/// deleting (`FUN_005f54a0`).
pub(super) fn apply_message_index_actions(
    actions: Vec<MessageIndexAction>,
    log: &mut MessageLog,
    clock: &mut GameClock,
    saved_speed: &mut Option<GameSpeed>,
) {
    for action in actions {
        match action {
            MessageIndexAction::AdviceShown => {
                rebellion_render::game_speed::hold_speed_for_advice(clock, saved_speed, true);
            }
            MessageIndexAction::AdviceHidden => {
                rebellion_render::game_speed::hold_speed_for_advice(clock, saved_speed, false);
            }
            MessageIndexAction::MarkCategoryRead(Some(rail)) => log.mark_read(rail),
            MessageIndexAction::MarkCategoryRead(None) => log.mark_all_read(),
            MessageIndexAction::DeleteSelected(ids) => log.delete_by_ids(&ids),
            MessageIndexAction::MessageDisplayed(id) => log.display_message(id),
            // The caller opened the object's window or the Encyclopedia.
            MessageIndexAction::Display { .. } | MessageIndexAction::Encyclopedia(_) => {}
            // Either answer drops the held Abort (vt +0x38 / +0x3c); the
            // caller sent it for the X.
            MessageIndexAction::ContinueMission { message }
            | MessageIndexAction::AbortMission { message, .. } => {
                log.answer_continue_question(message);
            }
            MessageIndexAction::ToggleSilent(mask) => {
                log.toggle_silent(mask);
                macroquad::logging::info!(
                    "[interface] message_index posting mask={:?} silent={}",
                    mask,
                    log.posting_option(mask) & rebellion_render::message_log::SILENT != 0
                );
            }
            MessageIndexAction::Close => {}
        }
    }
}

pub(super) fn apply_agent_command(
    agent: &mut PlayerAgent,
    status: &mut rebellion_render::status_window::StatusWindowState,
    targeting: &mut Option<Targeting>,
    command: rebellion_render::agent_menu::AgentCommand,
    source: &str,
) {
    use rebellion_render::agent_menu::AgentCommand;
    let (name, on) = match command {
        // FUN_00439d10 → FUN_0049e360 keeps the kind; FUN_0041d5e0(0) puts
        // the view into the target cursor (FUN_00429320, mode 2). The
        // accelerators need the view out of it (FUN_00422ce0).
        AgentCommand::Build(kind) => {
            if targeting.is_none() {
                *targeting = Some(Targeting::new(TargetOrder::AgentBuild(kind)));
                macroquad::logging::info!(
                    "[agent] command=build kind={:?} status=targeting source={}",
                    kind,
                    source
                );
            }
            return;
        }
        // FUN_0041d7f0 → FUN_0042a440 posts 0x468 type 0x1a with the empty
        // id, which the galaxy view always opens.
        AgentCommand::Objectives => {
            status.open(rebellion_render::status_rows::StatusObject::Objectives);
            macroquad::logging::info!(
                "[agent] command=objectives status=opened source={}",
                source
            );
            return;
        }
        // The same with the galaxy's id 0xf1000004.
        AgentCommand::GalaxyOverview => {
            status.open(rebellion_render::status_rows::StatusObject::GalaxyOverview);
            macroquad::logging::info!(
                "[agent] command=galaxy_overview status=opened source={}",
                source
            );
            return;
        }
        AgentCommand::ManageGarrisons => (
            "manage_garrisons",
            agent.toggle(AutomationModule::Garrisons),
        ),
        AgentCommand::ManageProduction => (
            "manage_production",
            agent.toggle(AutomationModule::Production),
        ),
        AgentCommand::AgentAdvice => ("agent_advice", agent.toggle_advice()),
        AgentCommand::TranslateCounterpart => ("translate_counterpart", agent.toggle_translate()),
    };
    macroquad::logging::info!(
        "[agent] command={name} status={} source={source}",
        if on { "on" } else { "off" }
    );
}
