use super::*;
use rebellion_data::integrator::PerceptionIntegrator;
use rebellion_data::simulation::{
    begin_simulation_day_with, close_simulation_day, run_ai_passes, run_simulation_tick, SliceRolls,
};

#[test]
#[ignore = "requires original data/base DAT files"]
fn the_app_s_day_with_its_presentation_matches_the_headless_runner() {
    // F-012: the app runs the runner's day (its start at the day's
    // event, one AI pass a sub-tick, FUN_0041dff0 phase 8, and its end
    // after the last) from the same day's rolls, with its messages as
    // hooks that change nothing; with no player input and no tactical
    // battle, a seed gives the same campaign.
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let world = rebellion_data::load_game_data_with_options(
        &data,
        &rebellion_data::replay_fixture::seed42_seed_options(),
    )
    .expect("load original campaign data");
    let initial = rebellion_data::replay_fixture::seed42_initial_state(world).unwrap();
    let days = 40;
    let runner = rebellion_data::replay::drive_days(
        initial.clone(),
        GameSpeed::Medium,
        true,
        days,
        &mut |world, states, ticks, rolls| {
            run_simulation_tick(world, states, ticks, rolls, 0);
        },
    )
    .unwrap();

    let message_texts = std::collections::HashMap::new();
    let mut log = MessageLog::default();
    let mut battled = std::collections::HashSet::new();
    let mut pending_ending = None;
    let mut player_agent = PlayerAgent::default();
    let mut advisor = AdvisorState::new(AdvisorFaction::Alliance);
    // Not the galaxy: no battle becomes a tactical battle.
    let mut game_mode = GameMode::MainMenu;
    let mut tactical_state = TacticalState::new();
    let mut battle_sorties = Vec::new();
    let mut other_fog = FogState::new(Faction::Empire);
    let mut assault_summary = rebellion_render::assault_summary::AssaultSummaryState::default();
    let app = rebellion_data::replay::drive_days(
        initial,
        GameSpeed::Medium,
        true,
        days,
        &mut |world, sim, ticks, rolls| {
            let mut presentation = AppDayPresentation {
                log: &mut log,
                battled: &mut battled,
                message_texts: &message_texts,
                cockpit_faction: CockpitFaction::Alliance,
                advisor_schedule: None,
                advisor_now: 0,
                pending_ending: &mut pending_ending,
                player_faction: MissionFaction::Alliance,
                player_agent: &mut player_agent,
                advisor: &mut advisor,
                stash: DayStash::default(),
                game_mode: &mut game_mode,
                tactical_state: &mut tactical_state,
                battle_sorties: &mut battle_sorties,
                tactical_flags: [true; 5],
                other_fog: &mut other_fog,
                assault_summary: &mut assault_summary,
                audio: None,
            };
            let mut integrator = PerceptionIntegrator::new(ticks[0].tick, 0);
            begin_simulation_day_with(world, sim, ticks, rolls, &mut integrator, &mut presentation);
            for upto in 1..=sim.ai_day.passes {
                run_ai_passes(world, sim, &mut integrator, &mut presentation, upto);
            }
            let cursor = sim.ai_day.roll_cursor as usize;
            close_simulation_day(
                world,
                sim,
                &mut SliceRolls::at(rolls, cursor),
                &mut integrator,
                &mut presentation,
            );
        },
    )
    .unwrap();
    assert!(!log.is_empty(), "the app's presentation posted its messages");
    assert_eq!(app, runner);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn a_session_replay_puts_each_order_back_where_the_app_gave_it() {
    // Stage 2 of docs/plans/2026-11-24-player-orders-in-replay.md: the app
    // gives an order between two AI passes of a day (its frame step after
    // a sub-tick) or between days; the session log keeps the day and the
    // passes done, and its replay must reach the app's state.
    use rebellion_data::player_command::{after_order, dispatch_fleet, dispatch_fleet_command};
    use rebellion_data::session::{OrderMoment, SessionLog};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let world = rebellion_data::load_game_data_with_options(
        &data,
        &rebellion_data::replay_fixture::seed42_seed_options(),
    )
    .expect("load original campaign data");
    let initial = rebellion_data::replay_fixture::seed42_initial_state(world).unwrap();

    // The Alliance's fleets and every system, in id order.
    let mut fleets: Vec<FleetKey> = initial
        .world
        .fleets
        .iter()
        .filter(|(_, f)| f.is_alliance && f.ship_count() > 0)
        .map(|(key, _)| key)
        .collect();
    fleets.sort_by_key(|&f| initial.world.fleet_serial(f));
    let mut systems: Vec<(u32, SystemKey)> =
        initial.world.systems.iter().map(|(k, s)| (s.dat_id.0, k)).collect();
    systems.sort_unstable();

    let mut log = SessionLog::default();
    log.restart(initial.clone());
    // drive_days sets the speed before day 1, as the player does.
    log.record_speed(initial.clock.tick, OrderMoment::BetweenDays, true, GameSpeed::Medium);
    let days = 12u64;
    let mut day = 0u64;
    let message_texts = std::collections::HashMap::new();
    let mut messages = MessageLog::default();
    let mut battled = std::collections::HashSet::new();
    let mut pending_ending = None;
    let mut player_agent = PlayerAgent::default();
    let mut advisor = AdvisorState::new(AdvisorFaction::Alliance);
    let mut game_mode = GameMode::MainMenu;
    let mut tactical_state = TacticalState::new();
    let mut battle_sorties = Vec::new();
    let mut other_fog = FogState::new(Faction::Empire);
    let mut assault_summary = rebellion_render::assault_summary::AssaultSummaryState::default();
    // A fleet move: the first fleet and system the move takes.
    let order = |world: &mut GameWorld,
                     sim: &mut rebellion_data::simulation::SimulationStates,
                     log: &mut SessionLog,
                     moment: OrderMoment,
                     skip: usize| {
        let (fleet, to) = fleets
            .iter()
            .flat_map(|&f| systems.iter().map(move |&(_, s)| (f, s)))
            .filter(|&(f, s)| {
                rebellion_core::movement::validate_fleet_dispatch(&sim.movement, world, f, s, true)
                    .is_ok()
            })
            .nth(skip)
            .expect("an Alliance fleet that can move");
        let command = dispatch_fleet_command(world, fleet, to, &[]);
        dispatch_fleet(world, &mut sim.movement, &mut sim.troop_transport, fleet, to, &[], true)
            .unwrap();
        after_order(world, &sim.manufacturing, &sim.troop_transport);
        log.record_order(sim.clock.tick, moment, true, command);
    };
    let app = rebellion_data::replay::drive_days(
        initial,
        GameSpeed::Medium,
        false,
        days,
        &mut |world, sim, ticks, rolls| {
            day += 1;
            let mut presentation = AppDayPresentation {
                log: &mut messages,
                battled: &mut battled,
                message_texts: &message_texts,
                cockpit_faction: CockpitFaction::Alliance,
                advisor_schedule: None,
                advisor_now: 0,
                pending_ending: &mut pending_ending,
                player_faction: MissionFaction::Alliance,
                player_agent: &mut player_agent,
                advisor: &mut advisor,
                stash: DayStash::default(),
                game_mode: &mut game_mode,
                tactical_state: &mut tactical_state,
                battle_sorties: &mut battle_sorties,
                tactical_flags: [true; 5],
                other_fog: &mut other_fog,
                assault_summary: &mut assault_summary,
                audio: None,
            };
            let mut integrator = PerceptionIntegrator::new(ticks[0].tick, 0);
            begin_simulation_day_with(world, sim, ticks, rolls, &mut integrator, &mut presentation);
            for upto in 1..=sim.ai_day.passes {
                run_ai_passes(world, sim, &mut integrator, &mut presentation, upto);
                // Day 3, after its third pass: an order mid-day.
                if day == 3 && upto == 3 {
                    order(world, sim, &mut log, OrderMoment::InDay { passes_done: 3 }, 0);
                }
            }
            let cursor = sim.ai_day.roll_cursor as usize;
            close_simulation_day(
                world,
                sim,
                &mut SliceRolls::at(rolls, cursor),
                &mut integrator,
                &mut presentation,
            );
            // Day 6, after it closed: an order between days, and a faster
            // game from day 7.
            if day == 6 {
                order(world, sim, &mut log, OrderMoment::BetweenDays, 1);
                sim.clock.set_speed(GameSpeed::Fast);
                log.record_speed(sim.clock.tick, OrderMoment::BetweenDays, true, GameSpeed::Fast);
            }
        },
    )
    .unwrap();
    assert_eq!(log.entries().len(), 4);
    assert_eq!(log.unrecorded(), 0);
    let replayed = log.replay(days, OrderMoment::BetweenDays).unwrap();
    assert_eq!(replayed.fingerprint().unwrap(), app);
}

#[test]
#[ignore = "requires original data/base DAT files and an audio device"]
fn the_app_s_orders_land_in_the_session_log_and_replay_to_the_same_state() {
    // Stage 2b: apply_panel_action keeps each order it carries out as its
    // replay command; the log's replay then reaches the app's state.
    use rebellion_data::session::{OrderMoment, SessionLog};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let world = rebellion_data::load_game_data_with_options(
        &data,
        &rebellion_data::replay_fixture::seed42_seed_options(),
    )
    .expect("load original campaign data");
    let initial = rebellion_data::replay_fixture::seed42_initial_state(world).unwrap();
    let mut state = initial.clone();
    let mut session = SessionLog::default();
    session.restart(initial.clone());

    // The orders: the first Alliance fleet's move, a ship order, a mission.
    let mut fleets: Vec<FleetKey> = state
        .world
        .fleets
        .iter()
        .filter(|(_, f)| f.is_alliance && f.ship_count() > 0)
        .map(|(k, _)| k)
        .collect();
    fleets.sort_by_key(|&f| state.world.fleet_serial(f));
    let mut systems: Vec<(u32, SystemKey)> =
        state.world.systems.iter().map(|(k, s)| (s.dat_id.0, k)).collect();
    systems.sort_unstable();
    let (fleet, to) = fleets
        .iter()
        .flat_map(|&f| systems.iter().map(move |&(_, s)| (f, s)))
        .find(|&(f, s)| {
            rebellion_core::movement::validate_fleet_dispatch(&state.movement, &state.world, f, s, true)
                .is_ok()
        })
        .unwrap();
    let mut characters: Vec<(u32, CharacterKey)> = state
        .world
        .characters
        .iter()
        .filter(|(_, c)| c.is_alliance)
        .map(|(k, c)| (c.dat_id.0, k))
        .collect();
    characters.sort_unstable();
    let mission = characters
        .iter()
        .flat_map(|&(_, c)| systems.iter().map(move |&(_, s)| (c, s)))
        .find(|&(c, s)| {
            let mut probe = state.clone();
            probe
                .missions
                .dispatch_guarded(
                    rebellion_core::missions::MissionRequest::single(
                        MissionKind::Diplomacy,
                        MissionFaction::Alliance,
                        c,
                        s,
                        None,
                        probe.clock.tick,
                    ),
                    &mut probe.world,
                )
                .is_ok()
        })
        .unwrap();
    let actions = vec![
        PanelAction::DispatchFleet {
            fleet,
            destination: to,
            troops: Vec::new(),
        },
        PanelAction::DispatchMission {
            kind: MissionKind::Diplomacy,
            faction: MissionFaction::Alliance,
            team: vec![MissionMember::Character(mission.0)],
            decoys: Vec::new(),
            target: mission.1,
            target_character: None,
            target_object: None,
            tick: state.clock.tick,
        },
    ];

    let mut map_state = GalaxyMapState::default();
    let mut fog = state.fog_alliance.clone();
    let mut msg_log = MessageLog::default();
    let mut player_faction = MissionFaction::Alliance;
    let mut dual_ai_mode = false;
    let mut mod_runtime =
        rebellion_data::mods::ModRuntime::discover(&std::env::temp_dir().join("or-no-mods"));
    let message_texts = std::collections::HashMap::new();
    let mut assault_summaries = rebellion_render::assault_summary::AssaultSummaryState::default();
    let mut audio_engine = audio::AudioEngine::new();
    let audio_vol = AudioVolumeState::default();
    for action in actions {
        let mut refusal = None;
        apply_panel_action(
            action,
            &mut state.world,
            &mut map_state,
            &mut state.manufacturing,
            &mut state.missions,
            &mut state.movement,
            &mut state.troop_transport,
            &mut fog,
            &mut state.ai,
            &mut msg_log,
            &mut player_faction,
            &mut state.clock,
            &mut dual_ai_mode,
            &mut state.ai2,
            &mut state.victory,
            state.campaign_config,
            &mut state.blockade,
            &state.events,
            &mut mod_runtime,
            &message_texts,
            &mut assault_summaries,
            &mut refusal,
            &mut session,
            OrderMoment::BetweenDays,
            &mut audio_engine,
            &audio_vol,
            std::path::Path::new("."),
        );
        assert!(refusal.is_none(), "the order went");
        rebellion_data::player_command::after_order(
            &mut state.world,
            &state.manufacturing,
            &state.troop_transport,
        );
    }
    assert_eq!(session.entries().len(), 2, "both orders were logged");
    assert_eq!(session.unrecorded(), 0);
    let replayed = session.replay(state.clock.tick, OrderMoment::BetweenDays).unwrap();
    let app = rebellion_data::replay::CampaignDayRunner::new(state);
    assert_eq!(replayed.fingerprint().unwrap(), app.fingerprint().unwrap());
}
