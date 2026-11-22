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
    assert!(log.len() > 0, "the app's presentation posted its messages");
    assert_eq!(app, runner);
}
