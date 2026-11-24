mod audio;
#[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
mod dev_commands;
#[cfg(any(test, all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
#[cfg_attr(
    all(test, not(target_arch = "wasm32")),
    expect(
        dead_code,
        reason = "Native tests cover fixture helpers; the browser bridge is called only by WASM."
    )
)]
mod interface_test_fixture;
#[cfg(any(target_arch = "wasm32", test))]
mod runtime_pack;
mod day_presentation;
use day_presentation::{AppDayPresentation, DayStash};
mod day_messages;
use day_messages::{push_character_health, AdvanceForceChange, advance_force_changes, push_advance_messages, system_joins_message, jabba_delivers_message, push_foiled_missions, push_personnel_arrivals, escape_character, escape_messages, headquarters_arrival_message, push_force_discoveries, return_place_name, report_names, report_title_and_text, informants_message, maintenance_message, research_complete_message, fleet_arrival_message, saboteurs_strike_message, ship_damaged, ship_repaired_message, squadron_full_strength_message, force_level, push_force_growth, uprising_message_rail, uprising_event, filed, system_audience, blockade_messages};
#[cfg(test)]
use day_messages::{blockade_event, joins_event};
#[cfg(any(test, all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
use day_messages::blockade_started_message;
mod outcomes;
use outcomes::{apply_mission_result, battle_winner, apply_space_combat_result, log_bombardment, log_assault, present_assault_reports, destroyed_planet_messages};
mod orders;
use orders::{FleetMoveContext, DefensesMoveContext, issue_defenses_move, FleetDropOrder, fleet_drop_order, ship_move_action, TeamMenuOrder, team_menu_order, product_confirmation, team_squadron_place, team_gates, team_confirmation, defenses_selection_team, issue_team_move, issue_fleet_move, confirm_character_move, confirm_ship_move, move_out_of_blockade_asks, confirm_troop_move, order_headquarters_move, confirm_squadron_move, confirm_special_force_move, order_gates, retire_check, bombard_team, UNTRACED_REFUSAL, OTHER_SIDE_REFUSAL, RefusedOrder, refusal_status};
mod advisor_actions;
use advisor_actions::{loaded_advisor_schedule, new_advisor_schedule, advisor_now, post_agent_advice, apply_briefing_action, abort_missions, FinderTarget, apply_message_index_actions, apply_agent_command};
mod victory_word;
use victory_word::{victory_word_movies, game_decided_messages, settle_victory_word_between_days, drop_left_ending};
#[cfg(test)]
use victory_word::victory_winner_is_alliance;
#[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
use advisor_actions::advisor_messages;
mod panel_actions;
use panel_actions::{apply_panel_action, band_destination};
mod tactical_flow;
#[cfg(any(test, all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
#[cfg_attr(
    all(test, not(target_arch = "wasm32")),
    expect(
        dead_code,
        reason = "Native tests cover fixture code decoding; the bridge is called only by WASM."
    )
)]
mod tactical_test_fixture;
#[cfg(target_arch = "wasm32")]
mod web_accessibility;
#[cfg(target_arch = "wasm32")]
mod web_replay;

use ::rand::Rng;
use ::rand::SeedableRng;
use macroquad::prelude::*;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rebellion_core::ai::{AIAction, AIState, AiFaction, FleetMoveReason};
use rebellion_core::ai_schedule::AiDay;
use rebellion_core::blockade::{BlockadeState, BlockadeSystem};
use rebellion_core::combat::CombatSide;
use rebellion_core::agent_automation::{AutomationModule, PlayerAgent};
use rebellion_core::dat::Faction;
use rebellion_core::delivery::DeliveryState;
use rebellion_core::economy::{EconomyEvent, EconomyState, EconomySystem};
use rebellion_core::events::EventState;
use rebellion_core::fleet_join::FleetMover;
use rebellion_core::fog::{FogState, FogSystem};
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey};
use rebellion_core::manufacturing::ManufacturingState;
use rebellion_core::missions::{
    MissionEffect, MissionFaction, MissionKind, MissionMember, MissionState,
};
use rebellion_core::movement::{
    fleet_move_confirms, fleet_move_enabled, fleet_transit_ticks, fleets_move_enabled,
    system_side_fleets, validate_fleets_dispatch, MovementState,
};
use rebellion_core::production_cycle::ProductionState;
use rebellion_core::repair::{RepairEvent, RepairState};
use rebellion_core::research::ResearchState;
use rebellion_core::tick::{GameClock, GameSpeed};
use rebellion_core::squadrons::SquadronPlace;
use rebellion_core::troop_transport::{RegimentLeg, RegimentTarget, TroopTransportState};
use rebellion_core::uprising::UprisingState;
use rebellion_core::victory::{VictoryState, VictorySystem};
use rebellion_core::world::{
    CampaignConfig, GameWorld, SeedDifficulty, SeedOptions, VictoryConditions,
};

use rebellion_render::build_selection::{
    draw_build_selection, BuildSelectionAction, BuildSelectionState,
};
use rebellion_render::fleet_finder::{draw_fleet_finder, FleetFinderAction, FleetFinderState};
use rebellion_render::message_index::{MessageIndexAction, MessageIndexState};
use rebellion_render::message_pictures as pictures;
use rebellion_render::message_sound::MessageSounds;
use rebellion_render::personnel_finder::{
    draw_personnel_finder, PersonnelFinderAction, PersonnelFinderState,
};
use rebellion_render::troop_finder::{draw_troop_finder, TroopFinderAction, TroopFinderState};
use rebellion_render::game_speed::{
    choose_game_speed, draw_day_readout, draw_game_speed_menu, draw_pause_alert, draw_time_bar,
    open_game_speed_menu_on_right_click, pause_alert_contains_screen_point, stepped_game_speed,
    GameSpeedUiState,
};
use rebellion_render::resource_monitors::{
    draw_maintenance_monitor, draw_material_monitors, maintenance_available,
};
use rebellion_render::mission_dialog::{
    draw_mission_dialog, MissionDialogAction, MissionDialogState,
};
use rebellion_render::move_confirmation::{
    draw_move_confirmation, ConfirmedOrder, MoveConfirmation, MoveConfirmationAction,
    MoveConfirmationState, ShipTeam,
};
use rebellion_render::object_menu::{
    draw_object_menu, MenuObject, ObjectMenuCommand, ObjectMenuState, OrderGates,
};
use rebellion_render::quadrant_icons::{facility_icon_has_managers, Quadrant};
use rebellion_render::status_rows::StatusSources;
use rebellion_render::status_window::{draw_status_window, StatusWindowAction, StatusWindowState};
use rebellion_render::system_window::fleet_label;
use rebellion_render::targeting::{
    capture_pointer, draw_targeting_cursor, mission_release_target, release_destination,
    destination_drop_target, MissionReleaseSources, ReleaseTarget, ReleaseWindows,
    TargetOrder, Targeting, TargetingEnd,
};
use rebellion_render::{
    advisor_combat_result, advisor_greet, advisor_manufacturing_complete,
    advisor_mission_result, advisor_uprising, draw_advisor,
    draw_cockpit_background, draw_cockpit_chrome, draw_cockpit_egui_layer,
    draw_encyclopedia, draw_galaxy_backdrop, draw_galaxy_map, draw_game_options,
    draw_game_setup, draw_gid_caption, draw_main_menu, draw_multiplayer_setup,
    draw_sector_windows, draw_system_windows, draw_tactical_view,
    handle_cockpit_egui_input, set_cockpit_viewport_clip, AdvisorFaction, AdvisorState,
    AssetRenderProfile, AudioVolumeState, BmpCache, CockpitButton, CockpitFaction, CockpitState,
    EncyclopediaState, GalaxyMapState, GameMessage, GameOptionsAction,
    GameOptionsOrigin, GameOptionsState, GameSetupAction, GameSetupState, MainMenuAction,
    MainMenuState, MessageCategory, MessageLog, MessageLogState,
    MessageRail, MultiplayerSetupAction, MultiplayerSetupState, MusicContext,
    OriginalEncyclopediaCatalog, OriginalEncyclopediaEntry, PanelAction, RailAudience,
    SectorWindowAction, SectorWindowState, SfxKind, SystemWindowAction, SystemWindowState,
    TacticalAction, TacticalState, TacticalTrenchRunOutcome, VideoError, VideoPlayer,
};
use rebellion_render::{draw_defenses_windows, DefensesWindowAction, DefensesWindowState};
use rebellion_render::{draw_fleet_windows, FleetWindowAction, FleetWindowState};
use rebellion_render::{draw_missions_windows, MissionsWindowAction, MissionsWindowState};

/// Top-level game mode state machine.
///
/// Controls which screen renders each frame. Transitions are handled
/// in the main loop by matching on actions returned from each screen.
#[derive(Debug, Clone, PartialEq)]
enum GameMode {
    /// Full-screen prerendered cutscene playback.
    Cutscene { kind: CutsceneKind },
    /// Title screen: New Game / Load Game / Quit.
    MainMenu,
    /// Historical head-to-head setup destination.
    MultiplayerSetup,
    /// Campaign configuration: galaxy size, difficulty, faction.
    GameSetup,
    /// The main strategy game: galaxy map + War Room panels.
    Galaxy,
    /// Original unified save, audio, and tactical-display options screen.
    GameOptions { origin: GameOptionsOrigin },
    /// 2D tactical combat view for player-involved battles.
    TacticalCombat,
}

/// Which cutscene is playing — determines post-cutscene transition.
#[derive(Debug, Clone, PartialEq)]
enum CutsceneKind {
    /// App state 1: MDATA.000, the LucasArts and Coolhand logos; its end
    /// starts state 2 (`FUN_004138a0`'s `0x405` case).
    Intro,
    /// App state 2: MDATA.001, the card, the title, the crawl and the opening
    /// cinematic → the main menu (state 3).
    IntroCinematic,
    /// State 7: MDATA.003 (side 1) or MDATA.004 when a side is chosen → the
    /// game starts (state 8).
    NewGame,
    /// The main menu's credits (command `0x73`, `FUN_00406000`): MDATA.005
    /// → the main menu.
    Credits,
    /// States 0x10..0x13: the game's end, MDATA.105..108 → the main menu
    /// (state 3), the game reset (`FUN_00425c90`).
    Ending(u32),
    /// Death Star trench-run result (201/202.webm) → resume tactical view.
    TrenchRun(TacticalTrenchRunOutcome),
    /// A story movie, MDATA.101..104 (`FUN_00488b90` → `0x482` with
    /// `wParam` 1 → `FUN_004019a0(+0x5c, +0x50)`), its caption under it →
    /// the same galaxy view, its music restarted (`FUN_00422ce0`'s `0x402`).
    Story(u32),
}

/// The string modules the tooltips read besides TEXTSTRA (`FUN_005fefd0`:
/// modules 1, 3 and 12), by DLL.
const TOOLTIP_STRING_MODULES: [(&str, rebellion_render::win_tooltip::StringModule); 3] = [
    ("TEXTCOMM.DLL", rebellion_render::win_tooltip::StringModule::Common),
    ("TEXTTACT.DLL", rebellion_render::win_tooltip::StringModule::Tactical),
    ("REBDLOG.DLL", rebellion_render::win_tooltip::StringModule::Dialog),
];

const INTRO_CUTSCENE: &str = "assets/references/ref-videos/000.webm";
const INTRO_CINEMATIC: &str = "assets/references/ref-videos/001.webm";
const CREDITS_CUTSCENE: &str = "assets/references/ref-videos/005.webm";

/// The movie state 7 plays for the chosen side: MDATA.003 for side 1 (the
/// Alliance, `+0x108 == 1`), MDATA.004 otherwise.
const fn new_game_cutscene(alliance: bool) -> &'static str {
    if alliance {
        "assets/references/ref-videos/003.webm"
    } else {
        "assets/references/ref-videos/004.webm"
    }
}

/// The movie the game's end plays (`FUN_00425c90`, states 0x10..0x13):
/// the Alliance player sees 105 for its victory and 106 for the Empire's;
/// the Empire player 108 for its victory and 107 for the Alliance's.
/// The Game Options as the original stores them (`DisplayOpts` and the
/// volumes, rebellion_data::settings).
fn current_settings(
    audio: &AudioVolumeState,
    options: &GameOptionsState,
) -> rebellion_data::settings::Settings {
    use rebellion_data::settings::display;
    let flags = [
        (audio.music_enabled(), display::MUSIC),
        (options.show_starfield, display::STARFIELD),
        (options.show_planet, display::PLANET),
        (options.show_pyrotechnics, display::PYROTECHNICS),
        (options.high_detail_models, display::HIGH_DETAIL),
        (options.display_holocube, display::HOLOCUBE),
    ];
    rebellion_data::settings::Settings {
        music_volume: audio.music_volume,
        effects_volume: audio.sfx_volume,
        display_options: flags
            .into_iter()
            .filter(|(set, _)| *set)
            .fold(0, |word, (_, bit)| word | bit),
    }
}

/// Restore stored Game Options.
fn apply_settings(
    settings: &rebellion_data::settings::Settings,
    audio: &mut AudioVolumeState,
    options: &mut GameOptionsState,
) {
    use rebellion_data::settings::display;
    audio.music_volume = settings.music_volume;
    audio.sfx_volume = settings.effects_volume;
    audio.music_muted = !settings.shows(display::MUSIC);
    audio.dirty = true;
    options.show_starfield = settings.shows(display::STARFIELD);
    options.show_planet = settings.shows(display::PLANET);
    options.show_pyrotechnics = settings.shows(display::PYROTECHNICS);
    options.high_detail_models = settings.shows(display::HIGH_DETAIL);
    options.display_holocube = settings.shows(display::HOLOCUBE);
}

/// `FUN_00425c90`: the ending's state from the victory conditions word,
/// the Empire's win (`0x100000`) tested first; `None` (state 0) without a
/// win.
const fn ending_from_conditions_word(word: u32, player_is_alliance: bool) -> Option<u32> {
    if word & rebellion_core::victory::WORD_EMPIRE_WINS != 0 {
        Some(ending_cutscene(false, player_is_alliance))
    } else if word & rebellion_core::victory::WORD_ALLIANCE_WINS != 0 {
        Some(ending_cutscene(true, player_is_alliance))
    } else {
        None
    }
}

const fn ending_cutscene(alliance_won: bool, player_is_alliance: bool) -> u32 {
    match (player_is_alliance, alliance_won) {
        (true, true) => 105,
        (true, false) => 106,
        (false, false) => 108,
        (false, true) => 107,
    }
}

/// Whether a text box holds the keys, so the galaxy view's own shortcuts
/// stay quiet: an open Finder's name box (each Finder's opener focuses its
/// window, which hands the focus to the box, e.g. `FUN_0042a640`, the
/// Troop Finder's slot `+0x14` case 7), a rename edit (`FUN_004ac7a0`
/// focuses it), or the Encyclopedia's Topic box while focused
/// (`encyclopedia-window.md`). Keys go to the focused edit in the original.
const fn text_box_holds_keys(finders_open: [bool; 4], renaming: bool, topic_box: bool) -> bool {
    finders_open[0]
        || finders_open[1]
        || finders_open[2]
        || finders_open[3]
        || renaming
        || topic_box
}

/// Whether a key ends the playing movie: Escape (and a left click) always
/// (`FUN_005f7240`); any other key too, except in the trench run's
/// movies, which ignore `WM_KEYDOWN` (`FUN_00421790`, ids 0x70/0x71).
const fn key_skips_cutscene(kind: &CutsceneKind, escape: bool) -> bool {
    escape || !matches!(kind, CutsceneKind::TrenchRun(_))
}
const TRENCH_RUN_SUCCESS_CUTSCENE: &str = "assets/references/ref-videos/201.webm";
const TRENCH_RUN_FAILURE_CUTSCENE: &str = "assets/references/ref-videos/202.webm";

/// Build the cutscene asset path for a story cutscene number.
fn story_cutscene_path(number: u32) -> String {
    format!("assets/references/ref-videos/{number}.webm")
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Open Rebellion — Star Wars Rebellion".to_string(),
        window_width: 1280,
        window_height: 800,
        window_resizable: true,
        ..Default::default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn original_game_dir() -> PathBuf {
    std::env::var_os("REBELLION_GAME_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("REBELLION_MDATA_DIR")
                .map(PathBuf::from)
                .and_then(|directory| directory.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| PathBuf::from("../star-wars-rebellion"))
}

/// An original DLL's path: in the original installation when it is there,
/// otherwise beside the DAT files, where a staged `GData` directory keeps it.
#[cfg(not(target_arch = "wasm32"))]
fn original_dll_path(game_dir: &Path, gdata_path: &Path, name: &str) -> PathBuf {
    let installed = game_dir.join(name);
    if installed.exists() {
        installed
    } else {
        gdata_path.join(name)
    }
}

/// The score the mode a finished cutscene returns to plays. Opening a
/// cutscene stops the music, so its end has to start this again. The
/// tactical battle has no score (`FUN_00450540` stops it at the Battle
/// Alert), so a trench run returns to silence.
fn music_context_after_cutscene(kind: &CutsceneKind) -> Option<MusicContext> {
    match kind {
        CutsceneKind::Intro | CutsceneKind::NewGame => None,
        CutsceneKind::IntroCinematic | CutsceneKind::Credits | CutsceneKind::Ending(_) => {
            Some(MusicContext::MainMenu)
        }
        CutsceneKind::TrenchRun(_) => None,
        CutsceneKind::Story(_) => Some(MusicContext::GalaxyMap),
    }
}

/// The galaxy view's next track (`FUN_0041d3b0` -> `FUN_0043a250`), rolled
/// from the app's own random numbers.
fn next_galaxy_track(
    score: &mut rebellion_render::GalaxyScore,
    player_is_alliance: bool,
) -> rebellion_render::MusicTrack {
    score.next(player_is_alliance, |n| macroquad::rand::rand() % n.max(1))
}

/// The systems the viewer's side holds and those the opponent holds
/// (system record bits 4 and 8, `FUN_004ebe40`, summed by
/// `FUN_004c6830` and `FUN_0049db10`). port: the true holder, as
/// `agent_build` reads it, not the side's copy of each system.
fn systems_held_by_viewer(world: &GameWorld, player_is_alliance: bool) -> (i32, i32) {
    let (own_side, opponent_side) = if player_is_alliance {
        (Faction::Alliance, Faction::Empire)
    } else {
        (Faction::Empire, Faction::Alliance)
    };
    world
        .systems
        .values()
        .fold((0, 0), |(own, opponent), system| match system.control.faction() {
            Some(side) if side == own_side => (own + 1, opponent),
            Some(side) if side == opponent_side => (own, opponent + 1),
            _ => (own, opponent),
        })
}

/// Start `context`'s score: the menu's MDATA.300, or in a game the galaxy
/// view's next pick (`FUN_00417520(FUN_0041d3b0())`), which replaces the
/// playing track.
#[cfg(not(target_arch = "wasm32"))]
fn start_context_score(
    engine: &mut audio::AudioEngine,
    context: MusicContext,
    score: &mut rebellion_render::GalaxyScore,
    player_is_alliance: bool,
    sounds_dir: &Path,
    volume: &rebellion_render::AudioVolumeState,
) {
    if context == MusicContext::GalaxyMap {
        let track = next_galaxy_track(score, player_is_alliance);
        engine.stop_music();
        engine.play_music(track, sounds_dir, volume);
        macroquad::logging::info!(
            "[audio] context=galaxy track=original mdata={} mood={:#x}",
            track.mdata().unwrap_or_default(),
            score.mood()
        );
    } else {
        engine.play_music_for_context(context, sounds_dir, volume);
    }
}

/// The browser's [`start_context_score`]: the main theme kept decoded
/// resumes where a movie halted it; any other track is decoded from the
/// runtime pack.
#[cfg(target_arch = "wasm32")]
fn start_context_score(
    engine: Option<&mut audio::AudioEngine>,
    context: MusicContext,
    score: &mut rebellion_render::GalaxyScore,
    player_is_alliance: bool,
    main_theme: Option<&[u8]>,
    tracks: &std::collections::HashMap<rebellion_render::MusicTrack, Vec<u8>>,
) {
    let Some(engine) = engine else {
        return;
    };
    if context != MusicContext::GalaxyMap {
        let track = rebellion_render::MusicTrack::MainTheme;
        if let (false, Some(bytes)) = (engine.current_track() == Some(track), main_theme) {
            engine.load_music_bytes(track, bytes);
        }
        return;
    }
    let track = next_galaxy_track(score, player_is_alliance);
    if let Some(bytes) = tracks.get(&track) {
        engine.load_music_bytes(track, bytes);
    } else {
        engine.stop_music();
    }
    // The next pick decodes meanwhile, so it starts on time after a short
    // screen between two movies or at this one's end.
    let upcoming = score.peek(player_is_alliance, |n| macroquad::rand::rand() % n.max(1));
    if let Some(bytes) = tracks.get(&upcoming) {
        engine.prepare_music_bytes(upcoming, bytes);
    }
    macroquad::logging::info!(
        "[audio] context=galaxy track=original mdata={} mood={:#x} loaded={} seconds={:.3}",
        track.mdata().unwrap_or_default(),
        score.mood(),
        tracks.contains_key(&track),
        tracks
            .get(&track)
            .and_then(|bytes| audio::wave_duration_seconds(bytes))
            .unwrap_or_default()
    );
}

/// Whether the browser may start its loaded score this frame. No score
/// plays under a movie: the main menu's starts with the menu's window
/// (`FUN_00405050` WM_CREATE: `FUN_004173c0`, `FUN_00417520`), which state 3
/// creates only after the intro's movies (`FUN_004138a0`'s 0x405: MDATA.000
/// (0x65) -> state 2, MDATA.001 (0x66) -> state 3), and the other movies
/// stop the music first (`FUN_00610c00(0)`; ghidra/notes/movie-flow.md).
/// After the side's movie the game's galaxy window starts it at its
/// creation (`FUN_00422ce0`: `FUN_004173c0`, `FUN_00422cb0`), so the
/// frames that load the campaign stay silent too.
#[cfg(any(test, target_arch = "wasm32"))]
fn score_may_start(mode: &GameMode) -> bool {
    !matches!(mode, GameMode::Cutscene { .. } | GameMode::GameSetup)
}

/// What the battle windows' score does this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenScore {
    /// Nothing changes.
    Keep,
    /// The window's own track starts (the Battle Alert's or the Battle
    /// Summary's, `FUN_00417610`).
    Play(rebellion_render::MusicTrack),
    /// The window closed: the galaxy view's or the menu's score starts
    /// again (the summary's close, `FUN_004727a0`: `FUN_00417640`, then
    /// `FUN_00417520(FUN_0041d3b0())`).
    Restore,
}

/// The battle windows' score for this frame, from the track the screen
/// asks for (`screen_music`) and the one it last started (`playing`,
/// updated here). A movie leaves both alone: the summary plays its track
/// only at its first paint (`FUN_00471200` -> `FUN_00470e70` clears
/// `+0x188`), so a story movie over it returns to the galaxy view's score
/// (`FUN_00422ce0`'s 0x402: `FUN_00422cb0`), not the summary's. Game
/// Options over a battle window holds it too: on return the window goes
/// on with its track, and a Restart to the main menu gives the menu its
/// own score.
fn screen_score(
    mode: &GameMode,
    screen_music: Option<rebellion_render::MusicTrack>,
    playing: &mut Option<rebellion_render::MusicTrack>,
) -> ScreenScore {
    match mode {
        GameMode::Cutscene { .. } => {
            // A summary that opened on the movie's own frame has painted
            // too: the 0x482 handler pumps messages until the animations
            // end before the movie starts.
            *playing = screen_music;
            return ScreenScore::Keep;
        }
        GameMode::MainMenu | GameMode::Galaxy | GameMode::TacticalCombat => {}
        GameMode::GameOptions { .. } | GameMode::GameSetup | GameMode::MultiplayerSetup => {
            return ScreenScore::Keep;
        }
    }
    if screen_music == *playing {
        return ScreenScore::Keep;
    }
    *playing = screen_music;
    match screen_music {
        Some(track) => ScreenScore::Play(track),
        None if matches!(mode, GameMode::MainMenu | GameMode::Galaxy) => ScreenScore::Restore,
        None => ScreenScore::Keep,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn common_edata_sibling(gdata_path: &Path) -> Option<PathBuf> {
    gdata_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.eq_ignore_ascii_case("GData"))
        .and_then(|_| gdata_path.parent())
        .map(|root| root.join("EData"))
}

fn configured_edata_path(gdata_path: &Path) -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = std::env::var_os("REBELLION_EDATA_DIR").map(PathBuf::from) {
            return path;
        }
        let nested = gdata_path.join("EData");
        if nested.is_dir() {
            return nested;
        }
        if let Some(sibling) = common_edata_sibling(gdata_path) {
            if sibling.is_dir() {
                return sibling;
            }
        }
        original_game_dir().join("EData")
    }

    #[cfg(target_arch = "wasm32")]
    {
        gdata_path.join("EData")
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "main_tests/audio_source_tests.rs"]
mod audio_source_tests;

#[cfg(test)]
mod text_box_key_tests {
    use super::text_box_holds_keys;

    #[test]
    fn a_finder_a_rename_or_the_focused_topic_box_holds_the_galaxy_keys() {
        // The original sends keys to the focused edit: each Finder's name
        // box (the System Finder's too, FUN_00461000 case 7), a rename edit
        // (FUN_004ac7a0) and the Encyclopedia's Topic box once pressed.
        assert!(!text_box_holds_keys([false; 4], false, false));
        for finder in 0..4 {
            let mut open = [false; 4];
            open[finder] = true;
            assert!(text_box_holds_keys(open, false, false), "finder {finder}");
        }
        assert!(text_box_holds_keys([false; 4], true, false));
        assert!(text_box_holds_keys([false; 4], false, true));
    }
}

#[cfg(test)]
mod edata_path_tests {
    use super::*;

    #[test]
    fn common_original_install_uses_sibling_edata_directory() {
        assert_eq!(
            common_edata_sibling(Path::new("/games/Rebellion/GData")),
            Some(PathBuf::from("/games/Rebellion/EData"))
        );
        assert_eq!(common_edata_sibling(Path::new("data/base")), None);
    }
}

/// Resolve the explicit native asset profile. Browser builds remain on the
/// original-parity profile until manifest-approved HD entries join the runtime
/// pack, so a missing enhancement can never alter browser parity evidence.
fn configured_asset_render_profile() -> AssetRenderProfile {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::var("OPEN_REBELLION_ASSET_PROFILE") {
            Ok(value) => AssetRenderProfile::parse(&value).unwrap_or_else(|| {
                eprintln!(
                    "[assets] unknown OPEN_REBELLION_ASSET_PROFILE={value:?}; using original-parity"
                );
                AssetRenderProfile::OriginalParity
            }),
            Err(_) => AssetRenderProfile::OriginalParity,
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        AssetRenderProfile::OriginalParity
    }
}

/// The Game Options window's six rows (`FUN_00407180`). Saves the data
/// layer holds in slots 6..9 (the removed legacy panel's) stay on disk
/// untouched and are not listed. An unreadable save's row stays empty
/// (`game_options_saves`: `FUN_00415cd0` keeps `FUN_00411730`'s defaults).
fn read_save_slots(saves_dir: &Path) -> Vec<rebellion_render::SaveSlotInfo> {
    use rebellion_render::game_options::SAVE_ROWS;
    rebellion_data::save::game_options_saves(saves_dir, SAVE_ROWS)
        .into_iter()
        .map(|meta| rebellion_render::SaveSlotInfo {
            slot: meta.slot,
            name: meta.name,
            timestamp: if meta.timestamp_secs == 0 {
                "Browser save".to_string()
            } else {
                let hours = (meta.timestamp_secs / 3600) % 24;
                let minutes = (meta.timestamp_secs / 60) % 60;
                format!("{hours:02}:{minutes:02}")
            },
            game_tick: meta.game_tick,
            player_is_alliance: meta.player_is_alliance,
        })
        .collect()
}

/// Settles the Game Options save whose commit was `status`: once it is no
/// longer pending the rows show what is stored and the window shows
/// `FUN_00408240`'s message, "Selected Game Saved Successfully" or
/// "Selected Game Failed To Save". Returns the slot and whether it saved.
/// A native save settles at once; a browser one when IndexedDB answers.
/// The original has no string for a full disk, so a quota failure shows
/// 0x1219 like any other and its cause goes to the log only.
fn settle_game_options_save(
    state: &mut GameOptionsState,
    save_slots: &mut Vec<rebellion_render::SaveSlotInfo>,
    saves_dir: &Path,
    status: rebellion_data::save::WriteStatus,
) -> Option<(usize, bool)> {
    use rebellion_data::save::WriteStatus;
    let slot = state.save_in_flight?;
    let saved = match status {
        WriteStatus::Pending => return None,
        WriteStatus::Committed => true,
        WriteStatus::QuotaExceeded | WriteStatus::Failed => false,
    };
    state.save_in_flight = None;
    // A failed commit put the slot back as it was.
    *save_slots = read_save_slots(saves_dir);
    state.refresh_saves(save_slots);
    state.saved_notice = Some(saved);
    if saved {
        state.error = None;
        state.saved_this_visit = true;
    } else {
        macroquad::logging::warn!("[save] slot={} commit failed: {:?}", slot, status);
    }
    Some((slot, saved))
}

struct LiveCampaign<'a> {
    world: &'a mut GameWorld,
    clock: &'a mut GameClock,
    manufacturing: &'a mut ManufacturingState,
    missions: &'a mut MissionState,
    events: &'a mut EventState,
    ai: &'a mut AIState,
    movement: &'a mut MovementState,
    fog_alliance: &'a mut FogState,
    fog_empire: &'a mut FogState,
    player_faction: &'a mut MissionFaction,
    blockade: &'a mut BlockadeState,
    uprising: &'a mut UprisingState,
    research: &'a mut ResearchState,
    victory: &'a mut VictoryState,
    economy: &'a mut EconomyState,
    sim_rng: &'a mut Xoshiro256PlusPlus,
    ai2: &'a mut Option<AIState>,
    repair: &'a mut RepairState,
    production: &'a mut ProductionState,
    autoscrap: &'a mut rebellion_core::autoscrap::AutoscrapState,
    informant: &'a mut rebellion_core::informant::InformantState,
    healing: &'a mut rebellion_core::healing::HealingState,
    force_discovery: &'a mut rebellion_core::force_discovery::ForceDiscoveryState,
    dagobah: &'a mut rebellion_core::dagobah::DagobahState,
    final_battle: &'a mut rebellion_core::final_battle::FinalBattleState,
    jabba: &'a mut rebellion_core::jabba::JabbaState,
    seat_of_power: &'a mut rebellion_core::seat_of_power::SeatOfPowerState,
    troop_transport: &'a mut TroopTransportState,
    deliveries: &'a mut DeliveryState,
    player_agent: &'a mut PlayerAgent,
    combat_cooldowns: &'a mut std::collections::HashMap<rebellion_core::ids::SystemKey, u64>,
    campaign_config: &'a mut CampaignConfig,
    ai_day: &'a mut AiDay,
    day_rolls: &'a mut Vec<f64>,
}

impl LiveCampaign<'_> {
    fn snapshot(&self) -> rebellion_data::save::SaveState {
        rebellion_data::save::SaveState {
            world: self.world.clone(),
            clock: self.clock.clone(),
            manufacturing: self.manufacturing.clone(),
            missions: self.missions.clone(),
            events: self.events.clone(),
            ai: self.ai.clone(),
            movement: self.movement.clone(),
            fog_alliance: self.fog_alliance.clone(),
            fog_empire: self.fog_empire.clone(),
            player_is_alliance: *self.player_faction == MissionFaction::Alliance,
            blockade: self.blockade.clone(),
            uprising: self.uprising.clone(),
            research: self.research.clone(),
            victory: self.victory.clone(),
            economy: self.economy.clone(),
            sim_rng: self.sim_rng.clone(),
            ai2: self.ai2.clone(),
            repair: self.repair.clone(),
            production: self.production.clone(),
            autoscrap: self.autoscrap.clone(),
            informant: self.informant.clone(),
            healing: self.healing.clone(),
            force_discovery: self.force_discovery.clone(),
            dagobah: self.dagobah.clone(),
            final_battle: *self.final_battle,
            jabba: *self.jabba,
            seat_of_power: *self.seat_of_power,
            combat_cooldowns: self.combat_cooldowns.clone(),
            campaign_config: *self.campaign_config,
            troop_transport: self.troop_transport.clone(),
            deliveries: self.deliveries.clone(),
            player_agent: self.player_agent.clone(),
            object_views: self.world.object_views.clone(),
            products: self.world.products.clone(),
            squadron_damage: rebellion_core::squadrons::damaged_squadrons(
                self.world,
                &self.troop_transport.squadrons,
            ),
            system_copy_bits: self.world.object_views.system_bits.clone(),
            system_copy_uprisings: self.world.object_views.system_uprisings.clone(),
            ai_command_planners: rebellion_data::save::command_planners(self.ai, self.ai2.as_ref()),
            ai_sector_links: rebellion_data::save::sector_links(self.ai, self.ai2.as_ref()),
            ai_day: *self.ai_day,
            day_rolls: self.day_rolls.clone(),
            research_planner_bits: rebellion_data::save::research_bits(self.ai, self.ai2.as_ref()),
            ai_module_a: rebellion_data::save::module_a(self.ai, self.ai2.as_ref()),
            ai_module9: rebellion_data::save::module9(self.ai, self.ai2.as_ref()),
            production_fleets: rebellion_data::save::production_fleets(
                self.manufacturing,
                &self.world.products,
            ),
            escape_requests: self.missions.escape_requests().to_vec(),
        }
    }

    fn restore(&mut self, state: rebellion_data::save::SaveState) {
        *self.world = state.world;
        *self.clock = state.clock;
        *self.manufacturing = state.manufacturing;
        *self.missions = state.missions;
        *self.events = state.events;
        *self.ai = state.ai;
        self.ai.module_c = state.ai_command_planners.0;
        *self.movement = state.movement;
        *self.fog_alliance = state.fog_alliance;
        *self.fog_empire = state.fog_empire;
        *self.player_faction = if state.player_is_alliance {
            MissionFaction::Alliance
        } else {
            MissionFaction::Empire
        };
        *self.blockade = state.blockade;
        *self.uprising = state.uprising;
        *self.research = state.research;
        *self.victory = state.victory;
        *self.economy = state.economy;
        *self.sim_rng = state.sim_rng;
        *self.ai2 = state.ai2;
        if let (Some(ai2), Some(second)) = (self.ai2.as_mut(), state.ai_command_planners.1) {
            ai2.module_c = second;
        }
        rebellion_data::save::restore_sector_links(
            self.ai,
            self.ai2.as_mut(),
            &state.ai_sector_links,
        );
        rebellion_data::save::restore_research_bits(
            self.ai,
            self.ai2.as_mut(),
            &state.research_planner_bits,
        );
        rebellion_data::save::restore_module_a(self.ai, self.ai2.as_mut(), &state.ai_module_a);
        rebellion_data::save::restore_module9(self.ai, self.ai2.as_mut(), &state.ai_module9);
        *self.repair = state.repair;
        *self.production = state.production;
        *self.autoscrap = state.autoscrap;
        *self.informant = state.informant;
        *self.healing = state.healing;
        *self.force_discovery = state.force_discovery;
        *self.dagobah = state.dagobah;
        *self.final_battle = state.final_battle;
        *self.jabba = state.jabba;
        *self.seat_of_power = state.seat_of_power;
        *self.combat_cooldowns = state.combat_cooldowns;
        *self.campaign_config = state.campaign_config;
        *self.troop_transport = state.troop_transport;
        *self.deliveries = state.deliveries;
        *self.player_agent = state.player_agent;
        self.world.object_views = state.object_views;
        self.world.object_views.system_bits = state.system_copy_bits;
        self.world.object_views.system_uprisings = state.system_copy_uprisings;
        *self.ai_day = state.ai_day;
        *self.day_rolls = state.day_rolls;
        self.world.products = state.products;
        rebellion_core::squadrons::restore_damaged_squadrons(
            self.world,
            &mut self.troop_transport.squadrons,
            &state.squadron_damage,
        );
    }
}

#[cfg(any(target_arch = "wasm32", test))]
const REQUIRED_WASM_DATA: &[&str] = &[
    "SECTORSD.DAT",
    "SYSTEMSD.DAT",
    "CAPSHPSD.DAT",
    "FIGHTSD.DAT",
    "TROOPSD.DAT",
    "MJCHARSD.DAT",
    "MNCHARSD.DAT",
];

#[cfg(any(target_arch = "wasm32", test))]
const OPTIONAL_WASM_DATA: &[&str] = &[
    "encyclopedia-source.json",
    "encyclopedia-source.manifest.json",
    "GNPRTB.DAT",
    "SDPRTB.DAT",
    "DEFFACSD.DAT",
    "MANFACSD.DAT",
    "PROFACSD.DAT",
    "SYFCCRTB.DAT",
    "SYFCRMTB.DAT",
    "CMUNEFTB.DAT",
    "CMUNAFTB.DAT",
    "CMUNEMTB.DAT",
    "CMUNALTB.DAT",
    "CMUNCRTB.DAT",
    "CMUNHQTB.DAT",
    "CMUNYVTB.DAT",
    "FACLCRTB.DAT",
    "FACLHQTB.DAT",
    "DIPLMSTB.DAT",
    "ESPIMSTB.DAT",
    "ASSNMSTB.DAT",
    "INCTMSTB.DAT",
    "DSSBMSTB.DAT",
    "ABDCMSTB.DAT",
    "RCRTMSTB.DAT",
    "RESCMSTB.DAT",
    "SBTGMSTB.DAT",
    "SUBDMSTB.DAT",
    "ESCAPETB.DAT",
    "FDECOYTB.DAT",
    "FOILTB.DAT",
    "INFORMTB.DAT",
    "CSCRHTTB.DAT",
    "UPRIS1TB.DAT",
    "UPRIS2TB.DAT",
    "RLEVADTB.DAT",
    "RESRCTB.DAT",
    "TDECOYTB.DAT",
    "MISSNSD.DAT",
    "SPECFCSD.DAT",
    "ABODESD.DAT",
    "FLEETSD.DAT",
];

#[cfg(test)]
mod wasm_data_manifest_tests {
    use super::{OPTIONAL_WASM_DATA, REQUIRED_WASM_DATA};

    #[test]
    fn legacy_fallback_contains_every_encyclopedia_catalog_table() {
        for required in [
            "SYSTEMSD.DAT",
            "CAPSHPSD.DAT",
            "FIGHTSD.DAT",
            "TROOPSD.DAT",
            "MJCHARSD.DAT",
            "MNCHARSD.DAT",
            "DEFFACSD.DAT",
            "MANFACSD.DAT",
            "PROFACSD.DAT",
            "MISSNSD.DAT",
            "SPECFCSD.DAT",
            "FLEETSD.DAT",
        ] {
            assert!(
                REQUIRED_WASM_DATA.contains(&required) || OPTIONAL_WASM_DATA.contains(&required),
                "legacy browser fallback omits Encyclopedia source table {required}",
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn draw_loading_progress(label: &str, loaded: usize, total: usize) {
    clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
    let text = if total == 0 {
        label.to_string()
    } else {
        format!("{label} ({loaded}/{total})")
    };
    let font_size = 24.0;
    let dims = measure_text(&text, None, font_size as u16, 1.0);
    draw_text(
        &text,
        (screen_width() - dims.width) / 2.0,
        screen_height() / 2.0,
        font_size,
        WHITE,
    );
    let bar_w = 300.0;
    let bar_h = 8.0;
    let bar_x = (screen_width() - bar_w) / 2.0;
    let bar_y = screen_height() / 2.0 + 20.0;
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, DARKGRAY);
    let ratio = if total == 0 {
        0.0
    } else {
        loaded as f32 / total as f32
    };
    draw_rectangle(bar_x, bar_y, bar_w * ratio, bar_h, GREEN);
}

#[cfg(target_arch = "wasm32")]
fn install_runtime_pack(
    bytes: Vec<u8>,
) -> Result<std::collections::HashMap<String, Vec<u8>>, String> {
    let pack_len = bytes.len();
    let mut pack =
        runtime_pack::parse_owned_runtime_pack(bytes).map_err(|error| error.to_string())?;
    for required in REQUIRED_WASM_DATA {
        if !pack.game_files.contains_key(*required) {
            return Err(format!("required entry is missing: {required}"));
        }
    }

    let encyclopedia_assets =
        runtime_pack::take_namespace(&mut pack.game_files, "encyclopedia/assets/");
    let encyclopedia_asset_count = encyclopedia_assets.len();
    let game_file_count = pack.game_files.len();
    let string_table: std::collections::HashMap<u16, String> = pack
        .game_files
        .remove("textstra.json")
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default();
    let message_texts: std::collections::HashMap<u16, String> = pack
        .game_files
        .remove("textstra-messages.json")
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default();
    let bitmap_count = pack.bitmaps.len();
    let audio_file_count = pack.audio_files.len();
    let advisor_frame_count = pack.advisor_frames.len();
    let tactical_mesh_count = pack.tactical_meshes.len();
    let tactical_texture_count = pack.tactical_textures.len();
    if bitmap_count == 0 {
        return Err("runtime pack contains no UI bitmaps".to_string());
    }

    let advisor_bitmaps = pack
        .bitmaps
        .iter()
        .filter(|(key, _)| {
            ["alsprite-dll/", "emsprite-dll/", "albrief-dll/", "embrief-dll/"]
                .iter()
                .any(|prefix| key.starts_with(prefix))
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    if let Some(legends) = pack.game_files.remove("gid-legends.json") {
        rebellion_data::set_gid_legends_json(&legends);
    }
    if let Some(advice) = pack.game_files.remove("textstra-advice.json") {
        rebellion_data::set_agent_advice_json(&advice);
    }
    // The tooltips' other string modules, extracted at build time.
    for (dll, module) in TOOLTIP_STRING_MODULES {
        let name = format!("{}.json", dll.trim_end_matches(".DLL").to_ascii_lowercase());
        let table: std::collections::HashMap<u16, String> = pack
            .game_files
            .remove(&name)
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        rebellion_render::win_tooltip::set_module_strings(module, table);
    }
    rebellion_data::set_message_texts(message_texts);
    rebellion_data::set_string_table(string_table);
    rebellion_render::set_encyclopedia_asset_cache(encyclopedia_assets);
    rebellion_data::set_file_cache(pack.game_files);
    rebellion_render::set_advisor_asset_cache(pack.advisor_frames, advisor_bitmaps);
    rebellion_render::set_bmp_cache(pack.bitmaps);
    rebellion_render::set_tactical_asset_cache(pack.tactical_meshes, pack.tactical_textures);
    macroquad::logging::info!(
        "runtime_asset_pack loaded game_files={} encyclopedia_assets={} ui_bitmaps={} advisor_frames={} audio_files={} tactical_meshes={} tactical_textures={} bytes={}",
        game_file_count,
        encyclopedia_asset_count,
        bitmap_count,
        advisor_frame_count,
        audio_file_count,
        tactical_mesh_count,
        tactical_texture_count,
        pack_len
    );
    Ok(pack.audio_files)
}

#[cfg(target_arch = "wasm32")]
async fn load_legacy_wasm_assets() {
    use std::collections::HashMap;

    let total = REQUIRED_WASM_DATA.len() + OPTIONAL_WASM_DATA.len();
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut loaded = 0;

    for &name in REQUIRED_WASM_DATA {
        let path = format!("data/base/{name}");
        match macroquad::file::load_file(&path).await {
            Ok(data) => {
                files.insert(name.to_string(), data);
                loaded += 1;
            }
            Err(error) => panic!("Required file {name} failed to load: {error:?}"),
        }
        draw_loading_progress("Loading game data…", loaded, total);
        next_frame().await;
    }

    for &name in OPTIONAL_WASM_DATA {
        let path = format!("data/base/{name}");
        if let Ok(data) = macroquad::file::load_file(&path).await {
            files.insert(name.to_string(), data);
        }
        loaded += 1;
        draw_loading_progress("Loading game data…", loaded, total);
        next_frame().await;
    }

    let string_table: HashMap<u16, String> =
        match macroquad::file::load_file("data/base/textstra.json").await {
            Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
            Err(_) => HashMap::new(),
        };
    rebellion_data::set_string_table(string_table);
    let message_texts: HashMap<u16, String> =
        match macroquad::file::load_file("data/base/textstra-messages.json").await {
            Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
            Err(_) => HashMap::new(),
        };
    rebellion_data::set_message_texts(message_texts);
    if let Ok(legends) = macroquad::file::load_file("data/base/gid-legends.json").await {
        rebellion_data::set_gid_legends_json(&legends);
    }
    if let Ok(advice) = macroquad::file::load_file("data/base/textstra-advice.json").await {
        rebellion_data::set_agent_advice_json(&advice);
    }
    rebellion_data::set_file_cache(files);

    #[derive(serde::Deserialize)]
    struct BmpEntry {
        dll: String,
        id: u32,
    }

    let Ok(manifest_bytes) = macroquad::file::load_file("data/ui/bmp-manifest.json").await else {
        eprintln!("WARNING: bmp-manifest.json not found — UI textures will be missing");
        return;
    };
    let entries: Vec<BmpEntry> = match serde_json::from_slice(&manifest_bytes) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!(
                "ERROR: bmp-manifest.json is malformed: {error} — UI textures will be missing"
            );
            Vec::new()
        }
    };
    let bmp_total = entries.len();
    let mut bmp_cache = HashMap::with_capacity(bmp_total);
    let mut fetch_failures = 0;

    for (index, entry) in entries.iter().enumerate() {
        let path = format!("data/ui/{}/BMP/{}.bmp", entry.dll, entry.id);
        match macroquad::file::load_file(&path).await {
            Ok(data) => {
                bmp_cache.insert(format!("{}/{}", entry.dll, entry.id), data);
            }
            Err(error) => {
                fetch_failures += 1;
                if fetch_failures <= 5 {
                    eprintln!("WARNING: failed to fetch {path}: {error:?}");
                }
            }
        }
        let bmp_loaded = index + 1;
        if bmp_total > 0 && (bmp_loaded % 50 == 0 || bmp_loaded == bmp_total) {
            draw_loading_progress("Loading UI assets…", bmp_loaded, bmp_total);
            next_frame().await;
        }
    }

    if fetch_failures > 0 {
        eprintln!(
            "WARNING: {fetch_failures}/{bmp_total} BMP fetches failed — some UI textures will be missing"
        );
    }
    eprintln!(
        "Loaded {} of {} UI BMPs through legacy per-file fallback",
        bmp_cache.len(),
        bmp_total
    );
    rebellion_render::set_bmp_cache(bmp_cache);
}

#[cfg(target_arch = "wasm32")]
async fn load_wasm_assets() -> std::collections::HashMap<String, Vec<u8>> {
    draw_loading_progress("Loading optimized runtime assets…", 0, 0);
    next_frame().await;

    match macroquad::file::load_file("data/runtime.orpk").await {
        Ok(bytes) => install_runtime_pack(bytes)
            .unwrap_or_else(|error| panic!("Invalid data/runtime.orpk: {error}")),
        Err(error) => {
            eprintln!(
                "WARNING: data/runtime.orpk unavailable ({error:?}); using legacy per-file loading"
            );
            load_legacy_wasm_assets().await;
            std::collections::HashMap::new()
        }
    }
}

/// Cache every glyph the Macroquad layers can draw in the current Galaxy view
/// before the first draw call for a new font size.
///
/// Macroquad 0.4.x may resize its shared font atlas while a render batch still
/// references the old texture. Pre-measuring the complete character set makes
/// any resize happen at the safe start of the frame instead. Egui uses its own
/// atlas and is unaffected.
fn prewarm_galaxy_font_sizes(
    world: &GameWorld,
    map_state: &GalaxyMapState,
    warmed_sizes: &mut HashSet<u16>,
) {
    // The map never zooms (rebellion_render::galaxy_camera), so its label
    // sizes are those of scale 1.
    let mut sizes = vec![14, 18];
    if map_state.show_sector_labels {
        sizes.push(16);
    }

    if sizes.iter().all(|size| warmed_sizes.contains(size)) {
        return;
    }

    let mut characters = std::collections::BTreeSet::new();
    for text in [
        "REBEL ALLIANCE — COMMAND CENTER",
        "GALACTIC EMPIRE — COMMAND BRIDGE",
        "0123456789d",
    ] {
        characters.extend(text.chars());
    }
    for (_, sector) in &world.sectors {
        characters.extend(sector.name.chars());
    }
    for (_, system) in &world.systems {
        characters.extend(system.name.chars());
    }
    let sample: String = characters.into_iter().collect();

    for size in sizes {
        if warmed_sizes.insert(size) {
            measure_text(&sample, None, size, 1.0);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #![expect(
        clippy::too_many_lines,
        reason = "Keep the existing main loop together; extracting phases is a separate refactor."
    )]
    // The build's commit, for bug reports: scripts/build-wasm.sh sets it.
    macroquad::logging::info!(
        "Open Rebellion build {}",
        option_env!("OPEN_REBELLION_COMMIT").unwrap_or("unknown")
    );
    // Accept an optional GData path as the first CLI argument.
    // On WASM there is no CLI, so always use the hardcoded default.
    #[cfg(not(target_arch = "wasm32"))]
    let gdata_path = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            PathBuf::from(&args[1])
        } else {
            let candidate = PathBuf::from("data/base");
            if candidate.join("SYSTEMSD.DAT").exists() {
                candidate
            } else {
                eprintln!("Usage: open-rebellion <path-to-GData>");
                eprintln!("  GData directory must contain .DAT files (SYSTEMSD.DAT, etc.)");
                std::process::exit(1);
            }
        }
    };
    #[cfg(target_arch = "wasm32")]
    let gdata_path = PathBuf::from("data/base");

    let asset_render_profile = configured_asset_render_profile();
    macroquad::logging::info!("[assets] render_profile={}", asset_render_profile.as_str());

    #[cfg(target_arch = "wasm32")]
    if web_replay::requested() {
        web_replay::run(&gdata_path).await;
    }

    // ── Load game data ─────────────────────────────────────────────────────
    // Native: filesystem read via load_game_data()
    // WASM: HTTP fetch via macroquad::file::load_file() into cache, then load_game_data()
    #[cfg(not(target_arch = "wasm32"))]
    let mut world = match rebellion_data::load_game_data(&gdata_path) {
        Ok(w) => w,
        Err(e) => {
            eprintln!(
                "Failed to load game data from {}: {}",
                gdata_path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    #[cfg(target_arch = "wasm32")]
    let (mut world, mut browser_audio_files) = {
        let audio_files = load_wasm_assets().await;
        let world = rebellion_data::load_game_data(&gdata_path)
            .unwrap_or_else(|error| panic!("Failed to parse game data: {error}"));
        (world, audio_files)
    };

    eprintln!(
        "Loaded: {} systems, {} sectors, {} ship classes, {} fighter classes, {} characters",
        world.systems.len(),
        world.sectors.len(),
        world.capital_ship_classes.len(),
        world.fighter_classes.len(),
        world.characters.len(),
    );

    // The original message titles and bodies, from the player's own
    // TEXTSTRA.DLL (or the table extracted from it for the browser).
    let message_texts = rebellion_data::message_texts(&gdata_path);
    // The battle report's sentences (TEXTSTRA RCDATA 0x7001..0x7016).
    rebellion_render::tactical_view::set_battle_texts(message_texts.clone());
    // The tooltips' texts (TEXTSTRA RT_STRING, FUN_006008e0's shared tip).
    rebellion_render::win_tooltip::set_strings(rebellion_data::ui_strings(&gdata_path));
    #[cfg(not(target_arch = "wasm32"))]
    for (dll, module) in TOOLTIP_STRING_MODULES {
        rebellion_render::win_tooltip::set_module_strings(
            module,
            rebellion_data::dll_strings(&gdata_path, dll),
        );
    }
    // The galaxy view's legend windows (STRATEGY.DLL RCDATA, FUN_004522f0).
    rebellion_render::set_gid_legends(rebellion_data::gid_legends(&gdata_path));
    // The agent's advice lists, Alliance then Empire (TEXTSTRA RCDATA
    // 0x6000 / 0x6800, FUN_0048b460).
    let agent_advice_records = [
        rebellion_data::agent_advice_records(&gdata_path, true),
        rebellion_data::agent_advice_records(&gdata_path, false),
    ];

    // The Galactic Encyclopedia is immutable reference data, not campaign
    // state. Rebuild it from the installed DAT/TEXTSTRA source so opening the
    // index after a save/load never changes the serialized world format.
    let mut original_encyclopedia_catalog =
        match rebellion_data::encyclopedia_catalog::load_encyclopedia_catalog(&gdata_path) {
            Ok(source) => {
                let category_counts: [usize; 7] = std::array::from_fn(|index| {
                    source
                        .entries_for(source.categories[index].command_id)
                        .len()
                });
                let entries = source
                    .listed_entries()
                    .map(|entry| OriginalEncyclopediaEntry {
                        object_id: entry.object_id,
                        name: entry.name.clone(),
                    })
                    .collect();
                let category_labels = source.categories.map(|category| category.label);
                let catalog = OriginalEncyclopediaCatalog::new(
                    source.title,
                    source.topic_label,
                    category_labels,
                    entries,
                );
                macroquad::logging::info!(
                    "[encyclopedia] source_catalog loaded entries={} categories=all:{},systems:{},ships:{},facilities:{},missions:{},troops:{},personnel:{}",
                    catalog.len(),
                    category_counts[0],
                    category_counts[1],
                    category_counts[2],
                    category_counts[3],
                    category_counts[4],
                    category_counts[5],
                    category_counts[6],
                );
                Some(catalog)
            }
            Err(error) => {
                macroquad::logging::error!(
                    "[encyclopedia] source_catalog unavailable error={}",
                    error
                );
                None
            }
        };
    // The topics per side (FUN_0045fa60, P66A), from the extracted source.
    // Without them the original window stays closed to production and the
    // replacement Encyclopedia remains.
    let original_encyclopedia_topics =
        |audience| match rebellion_data::encyclopedia_topics::installed_encyclopedia_topics(
            &gdata_path,
            audience,
        ) {
            Ok(topics) => Some({
                macroquad::logging::info!(
                    "[encyclopedia] topics loaded audience={:?} entries={} complete={}",
                    audience,
                    topics.entries.len(),
                    topics.complete_count()
                );
                topics
                    .entries
                    .into_iter()
                    .map(|entry| {
                        let topic = rebellion_render::OriginalEncyclopediaTopic {
                            edata: entry.artwork_filename.as_deref().and_then(
                                rebellion_render::OriginalEncyclopediaTopic::edata_number,
                            ),
                            title: entry.title,
                            body: entry.body,
                        };
                        (entry.object_id, topic)
                    })
                    .collect::<std::collections::HashMap<u32, _>>()
            }),
            Err(error) => {
                // Not an error: without the extracted source the replacement
                // window stays.
                macroquad::logging::info!(
                    "[encyclopedia] topics unavailable audience={:?} error={:#}",
                    audience,
                    error
                );
                None
            }
        };
    let original_encyclopedia_topics = [
        original_encyclopedia_topics(
            rebellion_data::encyclopedia_topics::EncyclopediaAudience::Alliance,
        ),
        original_encyclopedia_topics(
            rebellion_data::encyclopedia_topics::EncyclopediaAudience::Empire,
        ),
    ];

    // ── Mod Runtime ──────────────────────────────────────────────────────────
    // mods/ lives alongside data/, not inside it: data/base → data → repo root → mods/
    let mods_dir = gdata_path
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(std::path::Path::new("."))
        .join("mods");
    let mut mod_runtime = rebellion_data::mods::ModRuntime::discover(&mods_dir);
    if !mod_runtime.discovered.is_empty() {
        eprintln!(
            "Discovered {} mods ({} enabled)",
            mod_runtime.discovered.len(),
            mod_runtime.discovered.iter().filter(|m| m.enabled).count()
        );
        let mod_errors = mod_runtime.apply_enabled(&mut world);
        for err in &mod_errors {
            eprintln!("Mod error: {err:?}");
        }
    }

    // ── Game mode ─────────────────────────────────────────────────────────
    let mut game_mode = GameMode::MainMenu;
    let mut main_menu_state = MainMenuState::default();
    let mut game_setup_state = GameSetupState::default();
    let mut multiplayer_setup_state = MultiplayerSetupState::default();
    let mut pending_cockpit_start: Option<GameSetupAction> = None;
    let mut pending_victory_conditions = VictoryConditions::Standard;
    let mut campaign_generation = 0_u32;

    // ── Simulation state ────────────────────────────────────────────────────
    // Seedable RNG for deterministic simulation
    let rng_seed = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(42, |d| d.as_secs())
        }
        #[cfg(target_arch = "wasm32")]
        {
            (macroquad::time::get_time() * 1_000_000.0) as u64
        }
    };
    let mut sim_rng = Xoshiro256PlusPlus::seed_from_u64(rng_seed);
    let mut clock = GameClock::new();
    let mut mfg_state = ManufacturingState::new();
    let mut mission_state = MissionState::new();
    let mut event_state = EventState::new();
    let mut ai_state = AIState::new(AiFaction::Empire);
    let mut campaign_config = CampaignConfig::default();
    let mut dual_ai_mode = false;
    let mut secondary_ai_state: Option<AIState> = None;
    // The current day's AI passes, one a sub-tick (rebellion_core::ai_schedule).
    let mut ai_day = AiDay::default();
    // The current day's rolls, drawn from `sim_rng` at the day's start as
    // the headless runner draws them (SaveState::day_rolls).
    let mut day_rolls: Vec<f64> = Vec::new();
    let mut movement_state = MovementState::new();
    let mut fog_alliance_state = FogState::new(Faction::Alliance);
    let mut fog_empire_state = FogState::new(Faction::Empire);
    FogSystem::seed(&mut fog_alliance_state, &world);
    FogSystem::seed(&mut fog_empire_state, &world);
    // The planet squadrons up for the player's battle, landed when it ends.
    let mut battle_sorties: Vec<rebellion_core::squadrons::Sortie> = Vec::new();
    // The systems a battle was resolved at since the last blockade pass:
    // a blockade that starts there sets +0x88 bit 0x4000 (FUN_0050a9e0,
    // the battle bit changed in the same pass) and FUN_004960f0 is silent.
    let mut battled_systems: std::collections::HashSet<rebellion_core::ids::SystemKey> =
        std::collections::HashSet::new();
    let mut combat_cooldowns: std::collections::HashMap<rebellion_core::ids::SystemKey, u64> =
        std::collections::HashMap::new();
    let mut blockade_state = BlockadeState::new();
    let mut uprising_state = UprisingState::new();
    let mut research_state = ResearchState::new();
    let mut repair_state = RepairState::default();
    let mut production_state = ProductionState::new();
    let mut autoscrap_state = rebellion_core::autoscrap::AutoscrapState::default();
    let mut informant_state = rebellion_core::informant::InformantState::default();
    let mut healing_state = rebellion_core::healing::HealingState::default();
    let mut force_discovery_state = rebellion_core::force_discovery::ForceDiscoveryState::default();
    let mut dagobah_state = rebellion_core::dagobah::DagobahState::default();
    let mut final_battle_state = rebellion_core::final_battle::FinalBattleState::default();
    let mut jabba_state = rebellion_core::jabba::JabbaState::default();
    let mut seat_of_power_state = rebellion_core::seat_of_power::SeatOfPowerState::default();
    let mut troop_transport_state = TroopTransportState::default();
    let mut delivery_state = DeliveryState::new();
    let mut economy_state = EconomyState::default();
    // The player's agent: Manage Garrisons and Manage Production, both off
    // at a new game (FUN_00439950).
    let mut player_agent = PlayerAgent::default();
    let mut agent_menu_state = rebellion_render::agent_menu::AgentMenuState::default();
    let mut alerts_menu_state = rebellion_render::agent_menu::AlertsMenuState::default();
    // Find HQ systems for victory detection
    let alliance_hq = world
        .systems
        .iter()
        .find(|(_, s)| s.is_headquarters && s.control.is_controlled_by(Faction::Alliance))
        .map(|(k, _)| k);
    let empire_hq = world
        .systems
        .iter()
        .find(|(_, s)| s.is_headquarters && s.control.is_controlled_by(Faction::Empire))
        .map(|(k, _)| k);
    let mut victory_state = if let (Some(a), Some(e)) = (alliance_hq, empire_hq) {
        VictoryState::new(a, e)
    } else {
        // Fallback: use first two systems if HQs not marked
        let mut keys = world.systems.keys();
        let a = keys
            .next()
            .expect("world must have at least 2 systems for victory");
        let e = keys
            .next()
            .expect("world must have at least 2 systems for victory");
        VictoryState::new(a, e)
    };

    // Register scripted story events
    rebellion_core::story_events::define_story_events(&mut event_state, &world);

    // ── UI state ────────────────────────────────────────────────────────────
    let mut map_state = GalaxyMapState::default();
    let mut warmed_galaxy_font_sizes = HashSet::new();
    let mut msg_log = MessageLog::default();
    let mut log_state = MessageLogState::default();

    // ── War Room panel state ────────────────────────────────────────────────
    let mut player_faction = MissionFaction::Alliance;
    let mut build_selection_state = BuildSelectionState::default();
    let mut mission_dialog_state = MissionDialogState::default();
    let mut move_confirmation_state = MoveConfirmationState::default();
    let mut status_window_state = StatusWindowState::default();
    let mut fleet_finder_state = FleetFinderState::default();
    let mut troop_finder_state = TroopFinderState::default();
    let mut personnel_finder_state = PersonnelFinderState::default();
    let mut system_finder_state = rebellion_render::system_finder::SystemFinderState::default();
    // The open Message Index (window 0x0d) and the speed its Advice
    // category saved (FUN_00487ff0 +0x58).
    let mut message_index: Option<MessageIndexState> = None;
    let mut advice_saved_speed: Option<GameSpeed> = None;
    let mut enc_state = EncyclopediaState::new();
    let mut mod_manager_state = rebellion_render::ModManagerState::default();
    #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
    let mut command_palette_state = rebellion_render::CommandPaletteState::new();
    #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
    let palette_enabled = dev_commands::palette_enabled();
    // Palette and script commands waiting for the frame that can run them.
    #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
    let mut pending_interface: Vec<rebellion_render::InterfaceCommand> = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    let mut command_script = dev_commands::CommandScript::from_env();
    enc_state.set_edata_path(configured_edata_path(&gdata_path));
    enc_state.set_asset_profile(asset_render_profile);
    // HD upscaled PNGs live as a sibling of the base data directory.
    let hd_path = gdata_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("hd")
        .join("EData");
    enc_state.set_hd_path(hd_path);

    // Panel visibility (mutually exclusive left panels)
    let saves_dir = rebellion_data::save::default_saves_dir();
    let mut save_slots = read_save_slots(&saves_dir);

    // ── Tactical combat state ────────────────────────────────────────────────
    let mut tactical_state = TacticalState::new();

    // ── Cockpit chrome ───────────────────────────────────────────────────────
    let mut cockpit_state = CockpitState::new(CockpitFaction::Alliance);
    let mut game_speed_ui = GameSpeedUiState::default();
    // The player's orders given while a pause holds the clock; they act
    // once it runs (`rebellion_render::panels::admit_orders`).
    let mut held_orders: Vec<PanelAction> = Vec::new();
    // The session since the last new game or load, as a replay
    // (rebellion_data::session; started once the galaxy runs between days)
    // and the speed it last logged.
    let mut session_log = rebellion_data::session::SessionLog::default();
    let mut session_needs_start = false;
    let mut session_speed: Option<GameSpeed> = None;
    let mut sector_window_state = SectorWindowState::default();
    let mut system_window_state = SystemWindowState::default();
    let mut fleet_window_state = FleetWindowState::default();
    let mut defenses_window_state = DefensesWindowState::default();
    let mut assault_summary_state =
        rebellion_render::assault_summary::AssaultSummaryState::default();
    let mut missions_window_state = MissionsWindowState::default();
    // The object pop-up menu a right-click in a system window opens.
    let mut object_menu: Option<ObjectMenuState> = None;
    // Where a window selection's menu team stands: its system and its
    // squadrons' place (the targeting's TeamMove).
    let mut object_menu_place: Option<(SystemKey, SquadronPlace)> = None;
    // A Mission order waiting for its target (the galaxy view's mode 2).
    let mut targeting: Option<Targeting> = None;
    // Whether the system cursor is hidden behind the targeting cursor.
    let mut system_cursor_hidden = false;
    let mut bmp_cache = BmpCache::new();
    {
        // gdata_path is data/base; staged UI BMPs live at data/base/ui/
        let ui_path = gdata_path.join("ui");
        bmp_cache.set_base_path(&ui_path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tactical_runtime = ui_path
                .join("tactical-dll")
                .join("TACTICAL3D")
                .join("runtime");
            if tactical_runtime.is_dir() {
                match rebellion_render::install_native_tactical_assets(&tactical_runtime) {
                    Ok(()) => macroquad::logging::info!(
                        "[tactical_3d] installed complete native tactical corpus"
                    ),
                    Err(error) => macroquad::logging::warn!(
                        "[tactical_3d] native tactical assets rejected: {}",
                        error
                    ),
                }
            }
        }
        // HD PNG overrides at data/hd/{dll-name}/{resource_id}.png.
        let hd_ui_path = gdata_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("hd");
        bmp_cache.set_hd_path(hd_ui_path);
        bmp_cache.set_render_profile(asset_render_profile);
    }

    // ── Droid advisor ──────────────────────────────────────────────────────
    let mut advisor_state = AdvisorState::new(AdvisorFaction::Alliance);
    // The side's advisor schedule (advisor_schedule.rs): a new game's plays
    // the opening briefing. port: a loaded game's is not saved, so it has
    // none.
    let mut advisor_schedule: Option<rebellion_render::advisor_schedule::AdvisorSchedule> = None;
    // The briefing's caption under a marked system (FUN_0041d890).
    let mut briefing_caption: Option<u16> = None;
    {
        let sprite_dir = gdata_path.join("ui");
        advisor_state.set_sprite_dir(&sprite_dir);
    }

    // ── Audio state ─────────────────────────────────────────────────────────
    let mut audio_vol = AudioVolumeState::default();
    // OPEN_REBELLION_MUTE silences music, effects and cutscenes from launch,
    // for native acceptance runs (scripts/launch-native.sh).
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("OPEN_REBELLION_MUTE").is_ok_and(|value| env_flag_on(&value)) {
        audio_vol.muted = true;
        audio_vol.music_muted = true;
        audio_vol.dirty = true;
    }
    let mut game_options_state = GameOptionsState::default();
    // The Game Options kept from the last run (the original's registry
    // values, rebellion_data::settings). A muted acceptance run neither
    // reads nor writes them.
    let persist_settings = !audio_vol.muted;
    let mut stored_settings = persist_settings
        .then(|| rebellion_data::settings::load(&saves_dir))
        .flatten();
    if let Some(settings) = stored_settings {
        apply_settings(&settings, &mut audio_vol, &mut game_options_state);
    }
    let mut quit_requested = false;
    let sounds_dir = PathBuf::from("data/sounds");

    #[cfg(not(target_arch = "wasm32"))]
    let mut audio_engine = {
        let mut engine = audio::AudioEngine::new();
        if sounds_dir.exists() {
            engine.load_all(&sounds_dir);
        }
        let game_dir = original_game_dir();
        let common_dll = original_dll_path(&game_dir, &gdata_path, "COMMON.DLL");
        if common_dll.exists() {
            engine.load_original_menu_sfx(&common_dll);
        }
        let tactical_dll = original_dll_path(&game_dir, &gdata_path, "TACTICAL.DLL");
        if tactical_dll.exists() {
            engine.load_original_tactical_sfx(&tactical_dll);
        }
        engine.load_original_tactical_voice(
            &original_dll_path(&game_dir, &gdata_path, "VOICEFXA.DLL"),
            &original_dll_path(&game_dir, &gdata_path, "VOICEFXE.DLL"),
        );
        audio_vol.backend_available = engine.is_available();
        engine
    };

    #[cfg(target_arch = "wasm32")]
    let browser_main_theme = browser_audio_files.remove("music/main_theme.wav");
    // The original score after MDATA.300: the galaxy view's picks
    // (FUN_0043a250, indices 1..=9) and the battle windows' tracks
    // (FUN_0044f860, FUN_0046f360, 10..=15).
    #[cfg(target_arch = "wasm32")]
    let browser_screen_music: std::collections::HashMap<rebellion_render::MusicTrack, Vec<u8>> = (1..=15)
        .map(rebellion_render::MusicTrack::Original)
        .filter_map(|track| {
            browser_audio_files
                .remove(&format!("music/{}", audio::music_file(track)))
                .map(|bytes| (track, bytes))
        })
        .collect();
    #[cfg(target_arch = "wasm32")]
    let browser_menu_sfx: Vec<_> = audio::MENU_SFX_ASSETS
        .iter()
        .filter_map(|&(kind, path, _)| browser_audio_files.remove(path).map(|bytes| (kind, bytes)))
        .collect();
    #[cfg(target_arch = "wasm32")]
    let browser_tactical_sfx: Vec<_> = audio::TACTICAL_SFX_ASSETS
        .iter()
        .filter_map(|&(_, _, path, resource_id)| {
            browser_audio_files
                .remove(path)
                .map(|bytes| (resource_id, bytes))
        })
        .collect();
    #[cfg(target_arch = "wasm32")]
    let browser_tactical_voice: Vec<_> = audio::tactical_voice_assets()
        .into_iter()
        .filter_map(|(faction, path, resource_id)| {
            browser_audio_files
                .remove(&path)
                .map(|bytes| (faction, resource_id, bytes))
        })
        .collect();
    // The briefing DLLs' voices (advisor_clip.rs), keyed by staged DLL
    // directory and WAVE id, with their lengths.
    let mut advisor_voice_seconds: std::collections::HashMap<(String, u16), f32> =
        std::collections::HashMap::new();
    // The Message window's two display sounds (FUN_00469de0).
    let mut message_sound_player = rebellion_render::message_sound::MessageSoundPlayer::default();
    #[cfg(target_arch = "wasm32")]
    let browser_advisor_voices: Vec<(String, u16, Vec<u8>)> = {
        let keys: Vec<String> = browser_audio_files
            .keys()
            .filter(|key| key.starts_with("advisor/"))
            .cloned()
            .collect();
        keys.into_iter()
            .filter_map(|key| {
                let (directory, file) = key["advisor/".len()..].split_once('/')?;
                let wave = file.strip_suffix(".wav")?.parse().ok()?;
                let directory = directory.to_owned();
                let bytes = browser_audio_files.remove(&key)?;
                Some((directory, wave, bytes))
            })
            .collect()
    };
    #[cfg(target_arch = "wasm32")]
    for (directory, wave, bytes) in &browser_advisor_voices {
        advisor_voice_seconds.insert(
            (directory.clone(), *wave),
            rebellion_render::advisor_clip::wave_seconds(bytes),
        );
    }
    #[cfg(target_arch = "wasm32")]
    let mut browser_menu_audio = if browser_main_theme.is_some()
        || !browser_menu_sfx.is_empty()
        || !browser_tactical_sfx.is_empty()
        || !browser_tactical_voice.is_empty()
        || !browser_advisor_voices.is_empty()
    {
        // Initialise WebAudio and begin decoding before the first interaction.
        // Its resume handlers are then ready for the first user gesture.
        let mut engine = audio::AudioEngine::new();
        if let Some(bytes) = browser_main_theme.as_deref() {
            engine.load_music_bytes(rebellion_render::MusicTrack::MainTheme, bytes);
        }
        for (kind, bytes) in &browser_menu_sfx {
            engine.load_sfx_bytes(*kind, bytes);
        }
        for (resource_id, bytes) in &browser_tactical_sfx {
            engine.load_tactical_sfx_bytes(*resource_id, bytes);
        }
        for (faction, resource_id, bytes) in &browser_tactical_voice {
            engine.load_tactical_voice_bytes(*faction, *resource_id, bytes);
        }
        for (directory, wave, bytes) in &browser_advisor_voices {
            engine.load_advisor_voice_bytes(directory, *wave, bytes);
        }
        Some(engine)
    } else {
        None
    };
    #[cfg(target_arch = "wasm32")]
    let mut browser_menu_audio_requested = false;
    #[cfg(target_arch = "wasm32")]
    {
        audio_vol.backend_available = browser_menu_audio.is_some();
    }
    let mut tactical_music_active = false;
    // The selected unit's looping hum now held (`TacticalState::selection_hum`).
    let mut tactical_hum: Option<rebellion_core::tactical::TacticalSelectionHum> = None;
    // The tactical effects' five channels (`audio::TacticalEffectChannels`).
    let mut tactical_channels = audio::TacticalEffectChannels::default();
    let mut screen_music_playing: Option<rebellion_render::MusicTrack> = None;
    // The galaxy view's score (`FUN_0043a250`) and the day its advisor
    // mood was last set.
    let mut galaxy_score = rebellion_render::GalaxyScore::default();
    let mut galaxy_mood_day: Option<u64> = None;

    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let interface_fixture_request = interface_test_fixture::requested();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let plays_intro =
        interface_fixture_request.is_none() && tactical_test_fixture::requested().is_none();
    #[cfg(not(all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
    let plays_intro = true;
    // The fixtures' galaxies have no Alliance headquarters, so the victory
    // word's HQ bit queues movie 102 at once; only the story-movie
    // scenario plays story movies, so no other fixture opens one.
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let plays_story_movies = interface_fixture_request
        .is_none_or(|request| {
            matches!(
                request.scenario,
                interface_test_fixture::Scenario::StoryMovie
                    | interface_test_fixture::Scenario::EndingDefeat
                    | interface_test_fixture::Scenario::EndingVictory
                    | interface_test_fixture::Scenario::EndingAssaultCoruscant
            )
        });
    #[cfg(not(all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
    let plays_story_movies = true;
    // A fixture opens straight on its scene: it must not request the intro.
    // The playing story movie's caption (FUN_00421790's caption window).
    let mut story_caption: Option<String> = None;
    // The ending movie the victory check chose, waiting for the story movies
    // queued ahead of it.
    let mut pending_ending: Option<u32> = None;
    let mut cutscene_player = if plays_intro {
        open_cutscene(
            Path::new(INTRO_CUTSCENE),
            audio_vol.cutscene_volume(),
            #[cfg(not(target_arch = "wasm32"))]
            &mut audio_engine,
            // Nothing sounds yet; the decoding main theme waits for the menu.
            #[cfg(target_arch = "wasm32")]
            None,
        )
    } else {
        None
    };
    if cutscene_player.is_some() {
        game_mode = GameMode::Cutscene {
            kind: CutsceneKind::Intro,
        };
    }

    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    if let Some(request) = interface_fixture_request {
        cutscene_player = None;
        audio_vol.muted = true;
        audio_vol.music_muted = true;
        audio_vol.dirty = true;
        interface_test_fixture::apply(
            request,
            &mut world,
            &mut game_mode,
            &mut player_faction,
            &mut cockpit_state,
            &mut map_state,
            &mut movement_state,
            &mut mfg_state,
            &mut economy_state,
            &mut mission_state,
            &mut blockade_state,
            &mut sector_window_state,
            &mut system_window_state,
            &mut fleet_window_state,
            &mut defenses_window_state,
            &mut troop_transport_state,
            &mut uprising_state,
        );
        if let Some(menu) = interface_test_fixture::starting_menu(request) {
            main_menu_state = menu;
        }
        interface_test_fixture::place_deliveries(request, &world, &mut delivery_state);
        // The fixture's queued units are products from their orders on
        // (FUN_0052a140; rebellion_core::products).
        rebellion_core::products::reconcile_carrying(&mut world, &mfg_state, &|fleet| {
            troop_transport_state.carried_count(fleet)
        });
        // Every fixture's cockpit shows its own side's droids (the
        // Empire's IMP sprites, not the Alliance's).
        let side = if player_faction == MissionFaction::Alliance {
            AdvisorFaction::Alliance
        } else {
            AdvisorFaction::Empire
        };
        advisor_state.set_faction(side);
        if interface_test_fixture::runs_the_clock(request) {
            choose_game_speed(&mut clock, GameSpeed::Fast);
        }
        if let Some((headquarters, coruscant)) =
            interface_test_fixture::place_headquarters_victory(request, &mut world)
        {
            victory_state = VictoryState::new(headquarters, coruscant);
            campaign_config.victory_conditions = VictoryConditions::HeadquartersOnly;
            macroquad::logging::info!(
                "[fixture] headquarters_victory headquarters={} coruscant={}",
                world.systems[headquarters].name,
                world.systems[coruscant].name
            );
        }
        if interface_test_fixture::has_the_advisor(request) {
            // The clock-running fixtures give the player's advisor its
            // messages, without the opening briefing, so their
            // notifications reach its event slot (FUN_0048a060).
            advisor_schedule = advisor_messages(&gdata_path, side).map(|messages| {
                rebellion_render::advisor_schedule::AdvisorSchedule::from_save(
                    side,
                    messages,
                    &[],
                    true,
                )
            });
        }
        interface_test_fixture::reveal(
            request,
            &world,
            &mut fog_alliance_state,
            &mut fog_empire_state,
        );
        // port: the fixture's sides hold views of the other side's objects
        // in the systems their fog shows, as a v116 save's migration seeds
        // them (object_views::seed_from_fog).
        rebellion_core::object_views::seed_from_fog(
            &mut world,
            &troop_transport_state,
            &movement_state,
            [&fog_alliance_state, &fog_empire_state],
        );
        rebellion_core::object_views::seed_extras_from_fog(
            &mut world,
            &movement_state,
            &mission_state,
            [&fog_alliance_state, &fog_empire_state],
        );
        rebellion_core::object_views::seed_details_from_fog(
            &mut world,
            &movement_state,
            &mission_state,
            &troop_transport_state,
            [&fog_alliance_state, &fog_empire_state],
        );
        // port: a fixture's fog stands for what its side sees now, so its
        // copies of those systems start live, as before the copies; the
        // day's step freezes them once the clock runs and the side does not
        // see them (FUN_004f9a60).
        let revealed: Vec<(Faction, rebellion_core::ids::DatId)> = world
            .systems
            .iter()
            .flat_map(|(key, value)| {
                [&fog_alliance_state, &fog_empire_state]
                    .into_iter()
                    .filter(move |fog| fog.is_visible(key))
                    .map(move |fog| (fog.faction, value.dat_id))
            })
            .collect();
        for (side, id) in revealed {
            let views = &mut world.object_views;
            let systems = if side == Faction::Alliance {
                &mut views.alliance_systems
            } else {
                &mut views.empire_systems
            };
            if let Some(view) = systems.get_mut(&id) {
                view.live = true;
            }
            let extras = if side == Faction::Alliance {
                &mut views.alliance_extras
            } else {
                &mut views.empire_extras
            };
            if let Some(value) = extras.get_mut(&id) {
                value.live = true;
            }
        }
        interface_test_fixture::reveal_without_copy(request, &world, &mut fog_empire_state);
        interface_test_fixture::place_new_game(
            request,
            &mut world,
            &mut movement_state,
            &troop_transport_state,
            &mission_state,
        );
        // Character Killed for the Alliance (FUN_0048fdf0, 0x7164 /
        // 0x7365): Jan Dodonna, filed first so it lists below the scenario's
        // other Mission messages. He stays alive in the world, as the
        // scenario's Future Jedi names him too.
        if let Some(character) = interface_test_fixture::killed_character(request, &world) {
            world.characters[character].is_killed = true;
            push_character_health(&mut msg_log, &world, character, 0, &message_texts);
            world.characters[character].is_killed = false;
        }
        // Character Injured, then Healed, for the Alliance (FUN_0048fdf0,
        // 0x7160 / 0x7161 and 0x7162 / 0x7163): Leia, injured for the
        // message only, so the scenario's world is unchanged.
        if let Some(character) = interface_test_fixture::injured_character(request, &world) {
            world.characters[character].injury = 1;
            push_character_health(&mut msg_log, &world, character, 0, &message_texts);
            world.characters[character].injury = 0;
            push_character_health(&mut msg_log, &world, character, 0, &message_texts);
        }
        // Natural Disaster for the Alliance (FUN_0048e940, 0x7190 / 0x7191),
        // filed as the day's disaster is: on the Resource rail with the
        // incident picture.
        if let Some(system) = interface_test_fixture::disaster_system(request, &world) {
            if let Some((title, text)) = rebellion_render::story_messages::disaster_texts(
                &world.systems[system].name,
                true,
                &[],
                &message_texts,
            ) {
                msg_log.push(filed(
                    GameMessage::at_system(0, text, MessageCategory::Event, system)
                        .with_picture_if(Some(pictures::system_incident(true)))
                        .with_sounds(MessageSounds::disaster())
                        .with_advisor_events(0x1c, 0x1c)
                        .with_title(title),
                    MessageRail::Resource,
                    Some(RailAudience::side(true)),
                ));
            }
        }
        // Smuggling Benefits for the Alliance (FUN_00499130, 0x7062 /
        // 0x7066): smuggling begins on an Empire-held Yavin. Filed first, so
        // it lists last on the Popular Support rail.
        if let Some(system) = interface_test_fixture::smuggled_system(request, &world) {
            // The other side's system: the Empire's for the Alliance reader,
            // the Alliance's for the Empire's.
            let alliance_reader = player_faction == MissionFaction::Alliance;
            if let Some(message) = rebellion_render::story_messages::smuggling_message(
                &world.systems[system].name,
                Some(if alliance_reader {
                    rebellion_core::dat::Faction::Empire
                } else {
                    rebellion_core::dat::Faction::Alliance
                }),
                alliance_reader,
                0,
                5,
                0,
                &message_texts,
            ) {
                msg_log.push(message);
            }
        }
        // Filed first, so it lists last on the Manufacturing rail.
        if let Some((tech, level)) = interface_test_fixture::researched_class(request, &world) {
            if let Some(message) = research_complete_message(&world, true, tech, level, 0, &message_texts) {
                msg_log.push(message);
            }
        }
        if let Some((fleet, system)) = interface_test_fixture::arrived_fleet(request, &world) {
            msg_log.push(fleet_arrival_message(&world, fleet, system, true, 0, &message_texts));
        }
        if let Some(system) = interface_test_fixture::blockaded_system(request, &world) {
            let message = blockade_started_message(&world, system, false, true, 0, &message_texts);
            macroquad::logging::info!("[fixture] blockade at {:?}: {}", system, message.is_some());
            if let Some(message) = message {
                msg_log.push(message);
            }
        }
        if let Some((at, troop)) = interface_test_fixture::sabotaged_regiment(request, &world) {
            let victim = rebellion_core::missions::sabotage_victim(
                &world,
                rebellion_core::missions::MissionTarget::Troop(troop),
            );
            if let Some(message) = saboteurs_strike_message(&world, victim, at, 0, &message_texts) {
                msg_log.push(message);
            }
        }
        for (trainer, found) in interface_test_fixture::place_force_discoveries(request, &mut world) {
            if let Some(message) = rebellion_render::story_messages::force_user_discovered(
                &world,
                trainer,
                found,
                0,
                &message_texts,
            ) {
                msg_log.push(message);
            }
        }
        // Filed before the scenario's other Manufacturing reports, so it
        // lists under them.
        for unit in interface_test_fixture::maintenance_shortfalls(request) {
            if let Some(message) = maintenance_message(
                &world,
                None,
                unit,
                player_faction == MissionFaction::Alliance,
                0,
                &message_texts,
            ) {
                msg_log.push(message);
            }
        }
        interface_test_fixture::post_conflict_report(request, &mut msg_log);
        interface_test_fixture::post_story_messages(request, &world, &mut msg_log, &message_texts);
        interface_test_fixture::post_event_messages(request, &mut world, &mut mission_state, &mut msg_log);
        push_foiled_missions(
            &mut msg_log,
            &world,
            &interface_test_fixture::foiled_missions(request, &world),
            &message_texts,
        );
        push_personnel_arrivals(
            &mut msg_log,
            &world,
            &interface_test_fixture::personnel_arrivals(request, &world),
            &message_texts,
        );
        // Recruitment Done (FUN_0048ae40), Capital Ship Repaired and
        // Squadron at Full Strength (FUN_00499de0), for the player's side.
        if let Some(alliance) = interface_test_fixture::recruitment_done(request) {
            if let Some(message) =
                rebellion_render::story_messages::recruitment_done_message(alliance, 0, &message_texts)
            {
                msg_log.push(message);
            }
        }
        if let Some((fleet, index)) = interface_test_fixture::repaired_ship(request, &world) {
            if let Some(message) = ship_repaired_message(&world, fleet, index, 0, &message_texts) {
                msg_log.push(message);
            }
        }
        if let Some((place, index)) =
            interface_test_fixture::full_strength_squadron(request, &world)
        {
            if let Some(message) =
                squadron_full_strength_message(&world, place, index, 0, &message_texts)
            {
                msg_log.push(message);
            }
        }
        // A0 uprising-clean/test-day795-umgul-near-uprising: the reader's
        // own Umgul near uprising, filed before the joins so it lists
        // under them.
        // A0 informants-clean/test-day1457-selonia: the reader's informants
        // tell of Selonia; filed first, it lists under the others.
        for name in interface_test_fixture::informant_reports(request) {
            if let Some(message) = world
                .systems
                .iter()
                .find(|(_, system)| system.name == name)
                .and_then(|(key, _)| {
                    informants_message(
                        &world,
                        key,
                        player_faction == MissionFaction::Alliance,
                        0,
                        &message_texts,
                    )
                })
            {
                msg_log.push(message);
            }
        }
        for name in interface_test_fixture::near_uprisings(request) {
            let Some(at) = world
                .systems
                .iter()
                .find(|(_, system)| system.name == name)
                .map(|(key, _)| key)
            else {
                continue;
            };
            let holder = Some(Faction::of_alliance(player_faction == MissionFaction::Alliance));
            if let Some(message) = rebellion_render::story_messages::near_uprising_message(
                at,
                name,
                holder,
                player_faction == MissionFaction::Alliance,
                true,
                false,
                1,
                0,
                0,
                &message_texts,
            ) {
                msg_log.push(message.with_picture_if(holder.and_then(pictures::near_uprising)));
            }
        }
        // A0 2026-11-16-empire-hunt h-30..h-50: Uprising Begins / Ends
        // (FUN_00499460, notification 0x14b) as the live uprising path
        // files them, by the holder at filing.
        for (name, holder, begins) in interface_test_fixture::uprising_messages(request) {
            let Some(at) = world
                .systems
                .iter()
                .find(|(_, system)| system.name == name)
                .map(|(key, _)| key)
            else {
                continue;
            };
            let Some((title, body)) = rebellion_render::story_messages::uprising_texts(
                name,
                Some(holder),
                begins,
                &message_texts,
            ) else {
                continue;
            };
            let picture = if begins {
                Some(pictures::uprising_begins())
            } else {
                pictures::uprising_ends(holder)
            };
            let message = GameMessage::at_system(0, body, MessageCategory::Diplomacy, at)
                .with_picture_if(picture)
                .with_sounds(MessageSounds::uprising(begins))
                .with_advisor_events(
                    uprising_event(Some(holder), Faction::Alliance, begins),
                    uprising_event(Some(holder), Faction::Empire, begins),
                )
                .with_title(title);
            msg_log.push(filed(
                message,
                MessageRail::PopularSupport,
                Some(RailAudience::Both),
            ));
        }
        // A0 message-index/category-1-day228: Corellia, then Commenor, joins
        // the Empire, as the Alliance reads it.
        for (name, tick) in interface_test_fixture::systems_joining_the_empire(request) {
            if let Some(system) = world
                .systems
                .iter()
                .find(|(_, system)| system.name == name)
                .map(|(key, _)| key)
            {
                if let Some(message) = system_joins_message(
                    &world,
                    system,
                    Some(Faction::Alliance),
                    Some(Faction::Empire),
                    player_faction == MissionFaction::Alliance,
                    tick,
                    &message_texts,
                ) {
                    msg_log.push(message);
                }
            }
        }
        // A0 empire-game3/joins-enemy: Commenor joins the Alliance, as the
        // Empire reads it.
        for (name, tick) in interface_test_fixture::systems_joining_the_alliance(request) {
            if let Some(system) = world
                .systems
                .iter()
                .find(|(_, system)| system.name == name)
                .map(|(key, _)| key)
            {
                if let Some(message) = system_joins_message(
                    &world,
                    system,
                    Some(Faction::Empire),
                    Some(Faction::Alliance),
                    player_faction == MissionFaction::Alliance,
                    tick,
                    &message_texts,
                ) {
                    msg_log.push(message);
                }
            }
        }
        for result in interface_test_fixture::mission_results(request, &mut world) {
            apply_mission_result(
                &result,
                false,
                false,
                &mut world,
                &mut research_state,
                &mut msg_log,
                &message_texts,
                #[cfg(not(target_arch = "wasm32"))]
                Some((&mut audio_engine, &audio_vol)),
            );
        }
        interface_test_fixture::post_own_capture_message(request, &world, &mut msg_log, &message_texts);
        interface_test_fixture::post_capture_message(request, &world, &mut msg_log, &message_texts);
        interface_test_fixture::post_capture_messages(
            request,
            &mut world,
            &mut msg_log,
            &message_texts,
        );
        interface_test_fixture::post_jabba_delivers(
            request,
            &mut world,
            &mut msg_log,
            &message_texts,
        );
        let reports =
            interface_test_fixture::enemy_assault(request, &mut world, &mut troop_transport_state);
        if !reports.is_empty() {
            present_assault_reports(
                reports,
                &world,
                player_faction,
                &message_texts,
                &mut assault_summary_state,
                &mut msg_log,
                0,
            );
            // FUN_00469de0: Display on a battle report opens its Assault
            // Summary, as the message index does. An own assault's opened
            // at once (kind 7).
            let report = msg_log.messages().iter().rev().find_map(|message| {
                message.assault_report.clone().map(|report| (message.id, report))
            });
            if let Some((id, report)) = report.filter(|_| {
                assault_summary_state.window_count() == 0
                    && interface_test_fixture::opens_the_assault_report(request)
            }) {
                assault_summary_state.open(&world, *report, &message_texts, Some(id));
            }
        }
        // The AI plays the other side, as a new game sets it.
        ai_state = AIState::new(if player_faction == MissionFaction::Empire {
            AiFaction::Alliance
        } else {
            AiFaction::Empire
        });
        interface_test_fixture::open_mission_dialog(
            request,
            &world,
            &uprising_state,
            &research_state,
            &mut mission_dialog_state,
        );
    }
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let tactical_fixture_request = tactical_test_fixture::requested();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let tactical_fixture_failed = if let Some(request) = tactical_fixture_request {
        cutscene_player = None;
        audio_vol.muted = true;
        audio_vol.music_muted = true;
        audio_vol.dirty = true;
        match tactical_test_fixture::apply(
            request,
            &mut world,
            &mut troop_transport_state,
            &mut tactical_state,
            &mut combat_cooldowns,
            &mut msg_log,
            &mut player_faction,
            &mut cockpit_state,
            &mut game_mode,
        ) {
            Ok(sorties) => {
                // The planet squadrons the fixture's battle sent up land when
                // it ends, as a campaign battle's do.
                battle_sorties = sorties;
                // port: the fixture's fleets reveal their system as a new
                // game's do (FogSystem::seed), so the strategy windows
                // after the battle show the fleets that remain there.
                FogSystem::seed(&mut fog_alliance_state, &world);
                FogSystem::seed(&mut fog_empire_state, &world);
                // port: a campaign battle's sides held live copies of the
                // system while their fleets were there (FUN_004f9a60), and
                // the sector window reads the viewer's copy (FUN_0045bbb0,
                // ghidra/notes/fog-of-war.md): A0 TAC-07-C014 shows the
                // Imperial fleet and facility to the Alliance after its last
                // ship died. The fixture's world never ran that view step.
                rebellion_core::object_views::seed_from_fog(
                    &mut world,
                    &troop_transport_state,
                    &movement_state,
                    [&fog_alliance_state, &fog_empire_state],
                );
                // The queued story movie needs a clock that is not paused
                // (FUN_0041dc10); the summary itself holds the days.
                if request.summary_story_movie {
                    choose_game_speed(&mut clock, GameSpeed::Fast);
                }
                false
            }
            Err(error) => {
                tactical_test_fixture::emit_failed(request, &error);
                true
            }
        }
    } else {
        false
    };
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut interface_fixture_frames = 0_u32;
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut interface_fixture_emitted = tactical_fixture_failed;
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut fleet_move_watch = interface_test_fixture::FleetMoveWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut fleet_load_watch = interface_test_fixture::FleetLoadWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut enemy_assault_watch = interface_test_fixture::EnemyAssaultWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut speed_menu_watch = interface_test_fixture::SpeedMenuWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut quadrant_watch = interface_test_fixture::QuadrantWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut fleet_finder_watch = interface_test_fixture::FleetFinderWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut system_finder_watch = interface_test_fixture::SystemFinderWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let interface_fixture_active =
        interface_fixture_request.is_some() || tactical_fixture_request.is_some();
    #[cfg(not(all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
    let interface_fixture_active = false;

    // ── Apply Star Wars theme ────────────────────────────────────────────
    // Must happen inside the macroquad async context, after first frame init.
    let mut theme_applied = false;

    loop {
        let dt = get_frame_time();
        let mut targeting_cursor_drawn = false;
        // The battle has ended: its Battle Summary is a window of the
        // Command Center (FUN_0042a640), drawn over the live strategy screen.
        let summary_mode = battle_summary_mode(&game_mode, tactical_state.battle_summary_open());
        if summary_mode != game_mode {
            game_mode = summary_mode;
            macroquad::logging::info!("[tactical-results] summary_window owner=command_center");
        }
        // What the Battle Summary returns this frame, routed after the
        // frame's screen is drawn.
        let mut battle_summary_action: Option<TacticalAction> = None;

        #[cfg(target_arch = "wasm32")]
        if !browser_menu_audio_requested
            && (is_mouse_button_pressed(macroquad::input::MouseButton::Left)
                || is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Tab))
        {
            browser_menu_audio_requested = true;
        }

        // Apply theme on first frame (egui context exists after first next_frame)
        if !theme_applied {
            egui_macroquad::ui(|ctx| {
                rebellion_render::theme::load_fonts(ctx);
                rebellion_render::theme::apply_theme(ctx);
            });
            egui_macroquad::draw();
            theme_applied = true;
        }

        // FUN_00469de0: each displayed message stops the last one's sounds
        // (FUN_0046a6e0) and plays its own (+0x30 STRATEGY, +0x32 sprite),
        // at the effects volume like the advisor's voices.
        for sounds in msg_log.take_displayed_sounds() {
            let faction = if player_faction == MissionFaction::Alliance {
                rebellion_render::advisor::AdvisorFaction::Alliance
            } else {
                rebellion_render::advisor::AdvisorFaction::Empire
            };
            for command in message_sound_player.display(sounds, faction) {
                use rebellion_render::message_sound::MessageSoundCommand;
                match command {
                    MessageSoundCommand::Stop { directory, wave } => {
                        #[cfg(not(target_arch = "wasm32"))]
                        audio_engine.stop_advisor_voice(directory, wave);
                        #[cfg(target_arch = "wasm32")]
                        if let Some(engine) = browser_menu_audio.as_mut() {
                            engine.stop_advisor_voice(directory, wave);
                        }
                    }
                    MessageSoundCommand::Play { directory, wave } => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let key = (directory.to_owned(), wave);
                            if let std::collections::hash_map::Entry::Vacant(entry) =
                                advisor_voice_seconds.entry(key)
                            {
                                let path = gdata_path
                                    .join("ui")
                                    .join(directory)
                                    .join("WAVE")
                                    .join(format!("{wave}.wav"));
                                if let Ok(bytes) = std::fs::read(path) {
                                    audio_engine.load_advisor_voice_bytes(directory, wave, &bytes);
                                    entry.insert(rebellion_render::advisor_clip::wave_seconds(&bytes));
                                }
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let loaded = audio_engine.play_advisor_voice(directory, wave, &audio_vol);
                        #[cfg(target_arch = "wasm32")]
                        let loaded = browser_menu_audio.as_mut().is_some_and(|engine| {
                            engine.play_advisor_voice(directory, wave, &audio_vol)
                        });
                        macroquad::logging::info!(
                            "[message] display sound directory={} wave={} loaded={} muted={}",
                            directory,
                            wave,
                            loaded,
                            audio_vol.muted
                        );
                    }
                }
            }
        }

        // ── Advisor animation timer ────────────────────────────────────────
        // A window created, not raised, tells the agent (FUN_0045aac0 /
        // FUN_00429ce0 → FUN_0041d2d0 → FUN_0043a0b0), whose first opening
        // of each kind posts its advice.
        //
        // The agent's periodic check (Class_0065ad70_vt38 → FUN_00439bc0)
        // compares sub-ticks (FUN_004fcee0). hyp: the panel's update runs
        // every frame of the galaxy view, so the port checks each frame.
        {
            use rebellion_core::agent_advice::AdviceWindow;
            let alliance = player_faction == MissionFaction::Alliance;
            let records = &agent_advice_records[usize::from(!alliance)];
            let now = clock.sub_tick();
            for (window, created) in [
                (AdviceWindow::Sector, sector_window_state.take_created()),
                (AdviceWindow::System, system_window_state.take_created()),
                (AdviceWindow::Fleet, fleet_window_state.take_created()),
                (AdviceWindow::Defenses, defenses_window_state.take_created()),
                (AdviceWindow::Missions, missions_window_state.take_created()),
            ] {
                if created {
                    let posted = player_agent.advice_window_opened(window, now, records);
                    post_agent_advice(
                        &posted,
                        alliance,
                        clock.tick,
                        &message_texts,
                        &mut msg_log,
                        "window",
                    );
                }
            }
            if game_mode == GameMode::Galaxy {
                let posted = player_agent.advice_check(now, records);
                post_agent_advice(&posted, alliance, clock.tick, &message_texts, &mut msg_log, "period");
            }
        }

        // The advisor runs on the scheduler's sub-tick ([`advisor_now`])
        // while the galaxy view or a story movie shows ([`advisor_runs`]).
        if advisor_runs(&game_mode) {
            if let Some(schedule) = advisor_schedule.as_mut() {
                let today = advisor_now(&clock);
                schedule.set_translate(player_agent.translate());
                // The agent's toggles (FUN_00439d60) reach the advisor's
                // reactions (FUN_004c2dd0 / FUN_004c0d00).
                let toggles = player_agent.advisor_flags.take();
                if toggles != 0 {
                    macroquad::logging::info!("[advisor] reaction flags={:#x}", toggles);
                    schedule.add_flags(toggles);
                }
                // Delivered messages hand their codes to the advisor's
                // event slot (FUN_0048a060 → +0x14).
                for code in msg_log.take_advisor_events(player_faction == MissionFaction::Alliance)
                {
                    macroquad::logging::info!("[advisor] reaction event code={:#x}", code);
                    schedule.event(code, today);
                }
                // A refused order reaches the advisor (FUN_00487c90 → +0xc).
                for status in msg_log.take_refusals() {
                    macroquad::logging::info!(
                        "[advisor] reaction refusal status={:#x}/{:#x} decoys=false",
                        status.0,
                        status.1
                    );
                    schedule.refused(status, false, today);
                }
                schedule.update(dt, today, &mut advisor_state.clips);
                // The agent carries the advisor's state into the save
                // (FUN_004397a0).
                let (last_day, enabled) = schedule.saved();
                player_agent.set_advisor_state(last_day, enabled);
            }
        }
        if advisor_schedule.is_none() {
            // Without the advisor's messages the codes have no reader.
            let _ = msg_log.take_advisor_events(player_faction == MissionFaction::Alliance);
            let _ = msg_log.take_refusals();
        }
        if !interface_fixture_active || advisor_schedule.is_some() {
            advisor_state.update(dt);
        }
        for event in advisor_state.take_clip_events() {
            match event {
                rebellion_render::advisor_clip::ClipEvent::Voice { directory, wave } => {
                    let key = (directory.to_owned(), wave);
                    #[cfg(not(target_arch = "wasm32"))]
                    if !advisor_voice_seconds.contains_key(&key) {
                        let path = gdata_path
                            .join("ui")
                            .join(directory)
                            .join("WAVE")
                            .join(format!("{wave}.wav"));
                        if let Ok(bytes) = std::fs::read(path) {
                            audio_engine.load_advisor_voice_bytes(directory, wave, &bytes);
                            advisor_voice_seconds.insert(
                                key.clone(),
                                rebellion_render::advisor_clip::wave_seconds(&bytes),
                            );
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    let loaded = audio_engine.play_advisor_voice(directory, wave, &audio_vol);
                    #[cfg(target_arch = "wasm32")]
                    let loaded = browser_menu_audio
                        .as_mut()
                        .is_some_and(|engine| engine.play_advisor_voice(directory, wave, &audio_vol));
                    let seconds = advisor_voice_seconds.get(&key).copied().unwrap_or(0.0);
                    advisor_state.clips.voice_started(loaded, seconds);
                    macroquad::logging::info!(
                        "[advisor] clip voice directory={} wave={} seconds={:.3} loaded={} muted={}",
                        directory,
                        wave,
                        seconds,
                        loaded,
                        audio_vol.muted
                    );
                }
                rebellion_render::advisor_clip::ClipEvent::StopVoice { directory, wave } => {
                    #[cfg(not(target_arch = "wasm32"))]
                    let stopped = audio_engine.stop_advisor_voice(directory, wave);
                    #[cfg(target_arch = "wasm32")]
                    let stopped = browser_menu_audio
                        .as_mut()
                        .is_some_and(|engine| engine.stop_advisor_voice(directory, wave));
                    macroquad::logging::info!(
                        "[advisor] clip voice stopped directory={} wave={} loaded={}",
                        directory,
                        wave,
                        stopped
                    );
                }
                rebellion_render::advisor_clip::ClipEvent::Finished(action) => {
                    if let Some(schedule) = advisor_schedule.as_mut() {
                        schedule.clip_finished(action);
                    }
                    // FUN_004c3060 / FUN_004c0f60 kind 3: FUN_0041d830 with
                    // the player's side's word sets the ending's state
                    // (FUN_00425c90); FUN_0042db70 then starts it.
                    if action.kind == 3 {
                        if let Some(number) = ending_from_conditions_word(
                            victory_state.conditions_word,
                            player_faction == MissionFaction::Alliance,
                        ) {
                            pending_ending = Some(number);
                        }
                    }
                    let side = advisor_schedule
                        .as_ref()
                        .map_or(advisor_state.faction, |schedule| schedule.faction());
                    let shown = (action.kind == 2)
                        .then(|| {
                            rebellion_render::advisor_schedule::briefing_action(side, action.code)
                        })
                        .flatten()
                        .map(|briefing| {
                            apply_briefing_action(
                                briefing,
                                &world,
                                &mut cockpit_state,
                                &mut map_state,
                                &mut briefing_caption,
                            )
                        });
                    macroquad::logging::info!(
                        "[advisor] clip finished kind={} code={:#x} gid={:?} caption={:?} marked={} holds_input={}",
                        action.kind,
                        action.code,
                        shown.map(|_| cockpit_state.gid_mode.command_id()),
                        briefing_caption,
                        match shown {
                            Some(Some(system)) => format!("{system:#x}"),
                            Some(None) => "none".to_owned(),
                            None => "-".to_owned(),
                        },
                        advisor_schedule.as_ref().is_some_and(|schedule| schedule.holds_input())
                    );
                    // FUN_004c30c0 / FUN_004c0fc0 case 0xd: the Message
                    // Index (window 0xd) closes (FUN_0042d760); with advice
                    // on, the agent builds its list (FUN_00439f20) and the
                    // index opens on Advice (FUN_0041d770(1, 0x82)).
                    // hyp: destroying the window runs its close
                    // (FUN_0046a6a0).
                    if action.kind == 2 && action.code == 0xd {
                        if let Some(index) = message_index.take() {
                            apply_message_index_actions(
                                index.close(),
                                &mut msg_log,
                                &mut clock,
                                &mut advice_saved_speed,
                            );
                        }
                        if player_agent.advice() {
                            let alliance = player_faction == MissionFaction::Alliance;
                            let posted = player_agent
                                .build_advice(&agent_advice_records[usize::from(!alliance)]);
                            post_agent_advice(
                                &posted,
                                alliance,
                                clock.tick,
                                &message_texts,
                                &mut msg_log,
                                "briefing",
                            );
                            let mut index = MessageIndexState::new();
                            let actions = index.set_category(0x82);
                            message_index = Some(index);
                            apply_message_index_actions(
                                actions,
                                &mut msg_log,
                                &mut clock,
                                &mut advice_saved_speed,
                            );
                            macroquad::logging::info!(
                                "[interface] destination=message_index category=0x82 status=opened_by_briefing"
                            );
                        }
                    }
                }
            }
        }

        // The briefing's hooks (FUN_0041d9d0): a left or right press, or
        // Escape's release, asks to skip; the mouse buttons stay held until
        // the closing action and the keys until the skip.
        let briefing_holds_input = advisor_schedule
            .as_ref()
            .is_some_and(|schedule| schedule.holds_input());
        let briefing_holds_keys = advisor_schedule
            .as_ref()
            .is_some_and(|schedule| schedule.holds_keys());
        if briefing_holds_input
            && (is_mouse_button_pressed(MouseButton::Left)
                || is_mouse_button_pressed(MouseButton::Right)
                || (briefing_holds_keys && is_key_released(KeyCode::Escape)))
        {
            if let Some(schedule) = advisor_schedule.as_mut() {
                schedule.request_skip();
                macroquad::logging::info!("[advisor] briefing skip requested");
            }
        }
        map_state.input_held = briefing_holds_input;

        // ── Global keyboard shortcuts ───────────────────────────────────────
        if briefing_holds_keys {
        } else if let GameMode::Cutscene { ref kind } = game_mode {
            // FUN_005f7240: Escape or the left button ends the movie;
            // FUN_00421790's WM_KEYDOWN: any other key too, but in the trench
            // run's movies. A skip ends only the playing movie.
            let escape = is_key_pressed(KeyCode::Escape)
                || is_mouse_button_pressed(MouseButton::Left);
            let other = !get_keys_pressed().is_empty();
            if escape || (other && key_skips_cutscene(kind, false)) {
                if let Some(player) = cutscene_player.as_mut() {
                    player.stop();
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            if let GameMode::GameOptions { origin } = game_mode.clone() {
                if game_options_state.escape() == GameOptionsAction::Return {
                    game_mode = match origin {
                        GameOptionsOrigin::ShuttleCockpit => GameMode::MainMenu,
                        GameOptionsOrigin::CommandCenter => GameMode::Galaxy,
                        GameOptionsOrigin::TacticalBattle => GameMode::TacticalCombat,
                    };
                }
            } else if matches!(game_mode, GameMode::MultiplayerSetup) {
                game_mode = GameMode::MainMenu;
            } else if game_mode == GameMode::Galaxy {
                if tactical_state.battle_summary_open() {
                    // The summary has the focus (FUN_0042a640's SetFocus):
                    // its own key slot closes it (FUN_004710f0, 0x1b), drawn
                    // with the window; the campaign is not left.
                } else if targeting.is_some() {
                    // port: Escape cancels targeting, as command 0x15e does
                    // in mode 2; no traced key posts 0x15e.
                    targeting = None;
                } else if move_confirmation_state.is_open() {
                    // The window's own key slot answers Escape (FUN_0044f640).
                } else if build_selection_state.is_open() {
                    // Build Selection's key slot closes it (FUN_00438b60).
                } else if status_window_state.is_open() {
                    // The Status window closes on Escape itself.
                } else if fleet_finder_state.is_open() {
                    // The Fleet Finder's key slot closes it (FUN_00463360).
                } else if troop_finder_state.is_open()
                    || personnel_finder_state.is_open()
                    || system_finder_state.is_open()
                {
                    // The Troop, Personnel and System Finders close on
                    // Escape in their own key slots, as the Fleet Finder.
                } else if assault_summary_state.window_count() > 0 {
                    // The Assault Summary's key slot closes it (FUN_0044e180).
                } else if let Some(index) = message_index.take() {
                    // port: Escape closes the Message Index as its Close
                    // button (0x28) does; no traced key slot is recovered.
                    apply_message_index_actions(
                        index.close(),
                        &mut msg_log,
                        &mut clock,
                        &mut advice_saved_speed,
                    );
                } else if fleet_window_state.renaming() {
                    // The rename edit answers Escape by closing unissued.
                } else if object_menu.is_some() {
                    // Escape closes only the open object pop-up menu.
                    object_menu = None;
                } else if game_speed_ui.menu_anchor.is_some() {
                    // Escape closes only the open Game Speed menu.
                    game_speed_ui.menu_anchor = None;
                } else if agent_menu_state.anchor.is_some() {
                    // Escape closes only the open Agent menu.
                    agent_menu_state.anchor = None;
                } else if cockpit_state.gid_ui.menu_open {
                    cockpit_state.gid_ui.menu_open = false;
                    cockpit_state.gid_ui.category = None;
                } else {
                    game_mode = GameMode::MainMenu;
                    macroquad::logging::info!(
                        "[main_menu] returned_from_campaign generation={} audio_context=main_menu",
                        campaign_generation
                    );
                }
            } else if game_mode == GameMode::MainMenu && main_menu_state.registry_open() {
                // The Fleet Registry answers Escape by closing itself.
            } else {
                quit_requested = true;
            }
        }
        let mut message_index_key = false;
        // ── Galaxy-mode keyboard shortcuts (blocked during event screen) ────
        // The open Fleet Finder takes the keys for its name box, and a
        // rename edit for the name it holds.
        if game_mode == GameMode::Galaxy
            && !briefing_holds_keys
            && !text_box_holds_keys(
                [
                    fleet_finder_state.is_open(),
                    troop_finder_state.is_open(),
                    personnel_finder_state.is_open(),
                    system_finder_state.is_open(),
                ],
                fleet_window_state.renaming(),
                enc_state.topic_box_holds_keys(),
            )
        {
            // F6 (case 0x75) opens the Message Index on All.
            message_index_key = is_key_pressed(KeyCode::F6);
            if is_key_pressed(KeyCode::R) {
                map_state = GalaxyMapState::default();
            }
            // Game Speed keys from the manual's keyboard reference.
            let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
            if alt_down {
                let requested = if is_key_pressed(KeyCode::P) {
                    Some(GameSpeed::Paused)
                } else if is_key_pressed(KeyCode::KpAdd) {
                    stepped_game_speed(&clock, true)
                } else if is_key_pressed(KeyCode::KpSubtract) {
                    stepped_game_speed(&clock, false)
                } else {
                    None
                };
                if let Some(requested) = requested {
                    choose_game_speed(&mut clock, requested);
                }
                // The windows the frame's modal gate holds (see
                // `window_modal_open` below).
                let agent_keys_under_modal = fleet_finder_state.is_open()
                    || troop_finder_state.is_open()
                    || personnel_finder_state.is_open()
                    || system_finder_state.is_open()
                    || status_window_state.is_open()
                    || enc_state.open
                    || message_index.is_some()
                    || tactical_state.battle_summary_open();
                // Alt+G and Alt+U (commands 0xbc5/0xbc6 → menu items
                // 0x115/0x116, FUN_00422ce0) toggle Manage Garrisons and
                // Manage Production (FUN_00439d60).
                // Alt+A (0xbc9 → 0x11e) flips Agent Advice (FUN_00439e80).
                // Alt+H (0xbd7 → 0x114) shows the Objectives, Alt+O (0xbc4 →
                // 0x113) the Galaxy Overview, Alt+V (0xbc8 → 0x11b) flips
                // Translate Counterpart. Alt+B, Alt+T and Alt+F (0xbc1..0xbc3
                // → 0x110..0x112) start the agent's Build orders when the
                // view is not already targeting.
                for (key, command) in [
                    (
                        KeyCode::B,
                        rebellion_render::agent_menu::AgentCommand::Build(
                            rebellion_core::agent_build::BuildKind::Ships,
                        ),
                    ),
                    (
                        KeyCode::T,
                        rebellion_render::agent_menu::AgentCommand::Build(
                            rebellion_core::agent_build::BuildKind::Troops,
                        ),
                    ),
                    (
                        KeyCode::F,
                        rebellion_render::agent_menu::AgentCommand::Build(
                            rebellion_core::agent_build::BuildKind::Facilities,
                        ),
                    ),
                    (
                        KeyCode::G,
                        rebellion_render::agent_menu::AgentCommand::ManageGarrisons,
                    ),
                    (
                        KeyCode::U,
                        rebellion_render::agent_menu::AgentCommand::ManageProduction,
                    ),
                    (
                        KeyCode::A,
                        rebellion_render::agent_menu::AgentCommand::AgentAdvice,
                    ),
                    (
                        KeyCode::H,
                        rebellion_render::agent_menu::AgentCommand::Objectives,
                    ),
                    (
                        KeyCode::O,
                        rebellion_render::agent_menu::AgentCommand::GalaxyOverview,
                    ),
                    (
                        KeyCode::V,
                        rebellion_render::agent_menu::AgentCommand::TranslateCounterpart,
                    ),
                ] {
                    // A modal window (a Finder, the Battle Summary, ...)
                    // refuses these; Alt+1..9 below still answer
                    // (FUN_00422ce0 0x483, DAT_006be4e4).
                    let refused = agent_keys_under_modal
                        && !rebellion_render::agent_menu::accelerator_answers_under_modal_window(
                            command.accelerator_command(),
                        );
                    if is_key_pressed(key) && !refused {
                        apply_agent_command(
                            &mut player_agent,
                            &mut status_window_state,
                            &mut targeting,
                            command,
                            "accelerator",
                        );
                    }
                }
                // Alt+1..9 pick the GID displays (accelerator table 11).
                const DIGITS: [KeyCode; 9] = [
                    KeyCode::Key1,
                    KeyCode::Key2,
                    KeyCode::Key3,
                    KeyCode::Key4,
                    KeyCode::Key5,
                    KeyCode::Key6,
                    KeyCode::Key7,
                    KeyCode::Key8,
                    KeyCode::Key9,
                ];
                let digit = DIGITS.iter().position(|key| is_key_pressed(*key));
                if let Some(mode) = digit
                    .and_then(|index| {
                        rebellion_render::GidMode::from_accelerator_digit(index as u8 + 1)
                    })
                    .filter(|mode| cockpit_state.select_gid_from_keyboard(*mode))
                {
                    macroquad::logging::info!(
                        "[interface] command=0x{:x} destination=gid status=selected label={} source=accelerator",
                        mode.command_id(),
                        mode.label()
                    );
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_selected(request, cockpit_state.gid_mode);
                    }
                }
            }
            if is_key_pressed(KeyCode::Tab) {
                mod_manager_state.open = !mod_manager_state.open;
            }
            #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
            if palette_enabled && is_key_pressed(KeyCode::GraveAccent) {
                command_palette_state.toggle();
                if command_palette_state.open {
                    command_palette_state.refresh_interface(&world);
                }
            }
        }

        // ── Tick the clock (Galaxy mode only) ────────────────────────────────
        let galaxy_before_ticks = game_mode == GameMode::Galaxy;
        // A change between day steps (the player's orders run at once)
        // settles the word before the clock moves, so a win ends the game
        // on its own day (A0 2026-11-17 edited-saves-5 run 2: Coruscant
        // taken on day 94, still 94 at the victory film).
        if game_mode == GameMode::Galaxy {
            settle_victory_word_between_days(
                &mut victory_state,
                &world,
                campaign_config.victory_conditions,
                &message_texts,
                &mut msg_log,
                advisor_schedule.as_mut(),
                advisor_now(&clock),
                &mut pending_ending,
                player_faction == MissionFaction::Alliance,
                clock.tick,
            );
        }
        // A queued play-test day spends no real time: its AI passes run at
        // once.
        let queued_day = clock.queued_days() > 0;
        let tick_events = if galaxy_clock_runs(
            &game_mode,
            tactical_state.battle_summary_open(),
            victory_state.conditions_word,
        ) {
            clock.advance(dt)
        } else {
            vec![]
        };
        // The campaign's simulation states moved into the shared day
        // functions' bundle (rebellion_data::simulation::SimulationStates)
        // and back. The player's fog stands in its `fog`; the other side's
        // stays here.
        macro_rules! sim_pack {
            () => {
                rebellion_data::simulation::SimulationStates {
                    clock: std::mem::take(&mut clock),
                    manufacturing: std::mem::take(&mut mfg_state),
                    missions: std::mem::take(&mut mission_state),
                    events: std::mem::take(&mut event_state),
                    ai: std::mem::replace(&mut ai_state, AIState::new(AiFaction::Empire)),
                    ai2: secondary_ai_state.take(),
                    movement: std::mem::take(&mut movement_state),
                    fog: std::mem::replace(
                        &mut fog_alliance_state,
                        FogState::new(Faction::Alliance),
                    ),
                    blockade: std::mem::take(&mut blockade_state),
                    uprising: std::mem::take(&mut uprising_state),
                    research: std::mem::take(&mut research_state),
                    victory: victory_state.clone(),
                    economy: std::mem::take(&mut economy_state),
                    repair: std::mem::take(&mut repair_state),
                    production: std::mem::take(&mut production_state),
                    autoscrap: std::mem::take(&mut autoscrap_state),
                    informant: std::mem::take(&mut informant_state),
                    healing: std::mem::take(&mut healing_state),
                    force_discovery: std::mem::take(&mut force_discovery_state),
                    dagobah: std::mem::take(&mut dagobah_state),
                    final_battle: std::mem::take(&mut final_battle_state),
                    jabba: std::mem::take(&mut jabba_state),
                    seat_of_power: std::mem::take(&mut seat_of_power_state),
                    troop_transport: std::mem::take(&mut troop_transport_state),
                    deliveries: std::mem::take(&mut delivery_state),
                    combat_cooldowns: std::mem::take(&mut combat_cooldowns),
                    campaign_config,
                    ai_day,
                }
            };
        }
        macro_rules! sim_unpack {
            ($sim:expr) => {{
                let sim = $sim;
                clock = sim.clock;
                mfg_state = sim.manufacturing;
                mission_state = sim.missions;
                event_state = sim.events;
                ai_state = sim.ai;
                secondary_ai_state = sim.ai2;
                movement_state = sim.movement;
                fog_alliance_state = sim.fog;
                blockade_state = sim.blockade;
                uprising_state = sim.uprising;
                research_state = sim.research;
                victory_state = sim.victory;
                economy_state = sim.economy;
                repair_state = sim.repair;
                production_state = sim.production;
                autoscrap_state = sim.autoscrap;
                informant_state = sim.informant;
                healing_state = sim.healing;
                force_discovery_state = sim.force_discovery;
                dagobah_state = sim.dagobah;
                final_battle_state = sim.final_battle;
                jabba_state = sim.jabba;
                seat_of_power_state = sim.seat_of_power;
                troop_transport_state = sim.troop_transport;
                delivery_state = sim.deliveries;
                combat_cooldowns = sim.combat_cooldowns;
                campaign_config = sim.campaign_config;
                ai_day = sim.ai_day;
            }};
        }
        // The app's side of the shared day functions (AppDayPresentation);
        // `$now` is the advisor's clock, read before the states are packed.
        macro_rules! app_presentation {
            ($now:expr) => {
                AppDayPresentation {
                    log: &mut msg_log,
                    battled: &mut battled_systems,
                    message_texts: &message_texts,
                    cockpit_faction: cockpit_state.faction,
                    advisor_schedule: advisor_schedule.as_mut(),
                    advisor_now: $now,
                    pending_ending: &mut pending_ending,
                    player_faction,
                    player_agent: &mut player_agent,
                    advisor: &mut advisor_state,
                    stash: DayStash::default(),
                    game_mode: &mut game_mode,
                    tactical_state: &mut tactical_state,
                    battle_sorties: &mut battle_sorties,
                    tactical_flags: game_options_state.tactical_flags(),
                    other_fog: &mut fog_empire_state,
                    assault_summary: &mut assault_summary_state,
                    #[cfg(not(target_arch = "wasm32"))]
                    audio: Some((&mut audio_engine, &audio_vol)),
                }
            };
        }
        // The day's AI passes, shared with the headless runner
        // (rebellion_data::simulation::run_ai_passes), until `$upto` of the
        // day's have run; with `$close`, a day whose passes are done runs
        // its end (close_simulation_day). Returns whether a pass ended the
        // game.
        macro_rules! run_ai_day {
            ($upto:expr, $close:expr) => {{
                let mut ended = false;
                if ai_day.done < ai_day.passes.min($upto) {
                    let now = advisor_now(&clock);
                    let mut sim = sim_pack!();
                    let mut presentation = app_presentation!(now);
                    let mut integrator =
                        rebellion_data::integrator::PerceptionIntegrator::new(sim.ai_day.day, 0);
                    ended = rebellion_data::simulation::run_ai_passes(
                        &mut world,
                        &mut sim,
                        &mut integrator,
                        &mut presentation,
                        $upto,
                    );
                    sim_unpack!(sim);
                }
                if $close && ai_day.owes_close() {
                    macroquad::logging::info!(
                        "[ai] end day={} passes={} orders={} second_orders={}",
                        ai_day.day,
                        ai_day.passes,
                        ai_state.orders_today,
                        secondary_ai_state.as_ref().map_or(0, |ai| ai.orders_today)
                    );
                    // The day's end, shared with the headless runner
                    // (rebellion_data::simulation::close_simulation_day).
                    let now = advisor_now(&clock);
                    let mut sim = sim_pack!();
                    let mut presentation = app_presentation!(now);
                    let mut integrator =
                        rebellion_data::integrator::PerceptionIntegrator::new(sim.ai_day.day, 0);
                    let cursor = sim.ai_day.roll_cursor as usize;
                    rebellion_data::simulation::close_simulation_day(
                        &mut world,
                        &mut sim,
                        &mut rebellion_data::simulation::SliceRolls::at(&day_rolls, cursor),
                        &mut integrator,
                        &mut presentation,
                    );
                    sim_unpack!(sim);
                }
                ended
            }};
        }
        // The messages posted from here on carry the scheduler's sub-tick
        // (FUN_004c4ce0's +0x38 from FUN_004fcee0).
        msg_log.set_now(clock.sub_tick());

        // The session log keeps each speed change before the day it sets
        // the passes of (rebellion_data::session).
        if session_log.has_start() && session_speed != Some(clock.speed) {
            session_log.record_speed(
                clock.tick,
                order_moment(&ai_day),
                player_faction == MissionFaction::Alliance,
                clock.speed,
            );
            session_speed = Some(clock.speed);
        }

        if !tick_events.is_empty() {
            // The previous day's passes not yet run, then its end, come
            // before the new day (FUN_0051dea0 raises the day after the
            // last sub-tick).
            run_ai_day!(u32::MAX, true);

            let current_tick = tick_events.last().map_or(0, |event| event.tick);
            // ── The day's systems ahead of its AI: economy, yards,
            // deliveries, movement, battles (the player's own as a tactical
            // battle, AppDayPresentation::take_battle), occupation, fog,
            // missions, the story systems, escapes, events and the day's
            // timers. Shared with the headless runner
            // (rebellion_data::simulation::begin_day, F-012); the app shows
            // each step's events (AppDayPresentation).
            {
                let now = advisor_now(&clock);
                let mut sim = sim_pack!();
                let mut presentation = app_presentation!(now);
                let mut integrator =
                    rebellion_data::integrator::PerceptionIntegrator::new(current_tick, 0);
                // The day's rolls, as the headless runner draws them for a
                // day (rebellion_data::replay::REPLAY_ROLLS_PER_TICK).
                day_rolls = (0..usize::from(rebellion_data::replay::REPLAY_ROLLS_PER_TICK))
                    .map(|_| sim_rng.gen::<f64>())
                    .collect();
                // ── AI: each day has as many passes as the speed has
                // sub-ticks, one a sub-tick, the Alliance's then the
                // Empire's, both sides' orders applied before the next pass
                // (FUN_0041dff0 phase 8, FUN_0041cf20 -> FUN_004361a0;
                // rebellion_core::ai_schedule), after the day's timers.
                rebellion_data::simulation::begin_simulation_day_with(
                    &mut world,
                    &mut sim,
                    &tick_events,
                    &day_rolls,
                    &mut integrator,
                    &mut presentation,
                );
                sim_unpack!(sim);
            }
        }

        // ── The day's AI passes, one as each sub-tick begins ─────────────
        // (FUN_0041dff0 phase 8; a day has rate + 1 sub-ticks, FUN_0051dea0).
        // A held or paused clock holds the remaining passes; a queued
        // play-test day runs them at once. The last pass's day then ends.
        if galaxy_clock_runs(
            &game_mode,
            tactical_state.battle_summary_open(),
            victory_state.conditions_word,
        ) {
            let upto = if queued_day {
                u32::MAX
            } else {
                ai_day.due(clock.day_part())
            };
            run_ai_day!(upto, true);
        }

        // The Battle Alert opens over the strategy view as it stood (A0
        // battle-clean/00). port: the frame a battle begins in still draws
        // the galaxy and is copied once at its end, where the battle takes
        // the screen; a copy every galaxy frame held Firefox at 3 fps.
        let battle_underlay_pending =
            galaxy_before_ticks && game_mode == GameMode::TacticalCombat;
        if battle_underlay_pending {
            game_mode = GameMode::Galaxy;
        }

        // ── Developer commands ───────────────────────────────────────────────

        let mut panel_actions: Vec<PanelAction> = Vec::new();
        // A command script runs one line a frame from the main menu or the
        // galaxy, once the last line's commands have run.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(script) = command_script.as_mut() {
            let ready = matches!(game_mode, GameMode::MainMenu | GameMode::Galaxy)
                && pending_interface.is_empty();
            match script.step(ready) {
                dev_commands::ScriptStep::Wait => {}
                dev_commands::ScriptStep::Run(line) => {
                    command_palette_state.refresh_script(&world);
                    match command_palette_state
                        .command_named(&line)
                        .map(|command| command.action.clone())
                    {
                        Some(rebellion_render::PaletteAction::Panel(action)) => {
                            eprintln!("[dev-command] sent {line:?}");
                            panel_actions.push(action);
                        }
                        Some(rebellion_render::PaletteAction::Interface(command)) => {
                            eprintln!("[dev-command] sent {line:?}");
                            pending_interface.push(command);
                        }
                        None => eprintln!("[dev-command] unknown {line:?}"),
                    }
                }
                dev_commands::ScriptStep::Done => {
                    eprintln!("[dev-command] done");
                    command_script = None;
                }
            }
        }
        // A start runs from the main menu as its faction control does;
        // everything else waits for the galaxy.
        #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
        let mut scripted_menu_action = None;
        #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
        if !matches!(game_mode, GameMode::Galaxy) {
            for command in std::mem::take(&mut pending_interface) {
                match command {
                    rebellion_render::InterfaceCommand::StartGame(faction)
                        if matches!(game_mode, GameMode::MainMenu) =>
                    {
                        scripted_menu_action = main_menu_state.activate_control(
                            if faction == MissionFaction::Alliance {
                                rebellion_render::MainMenuControl::Alliance
                            } else {
                                rebellion_render::MainMenuControl::Empire
                            },
                        );
                    }
                    _ => eprintln!("[dev-command] refused: the galaxy is not showing"),
                }
            }
        }

        // The game's end, after the story movies ahead of it
        // (pending_ending).
        if game_mode == GameMode::Galaxy
            && !battle_underlay_pending
            && cutscene_player.is_none()
            && (!plays_story_movies || msg_log.queued_story_movies().next().is_none())
        {
            if let Some(number) = pending_ending.take() {
                macroquad::logging::info!("[cutscene] ending={number} day={}", clock.tick);
                game_mode = GameMode::MainMenu;
                // The film's end (0x405, ids 0x6a..0x6d) is app state 3,
                // which builds the cockpit anew (FUN_00414a80).
                main_menu_state = main_menu_state.rebuilt();
                macroquad::logging::info!(
                    "[main_menu] rebuilt headquarters_only={} difficulty={:?} galaxy={:?}",
                    main_menu_state.headquarters_only,
                    main_menu_state.difficulty,
                    main_menu_state.galaxy_size
                );
                cutscene_player = open_cutscene(
                    Path::new(&story_cutscene_path(number)),
                    audio_vol.cutscene_volume(),
                    #[cfg(not(target_arch = "wasm32"))]
                    &mut audio_engine,
                    #[cfg(target_arch = "wasm32")]
                    browser_menu_audio.as_mut(),
                );
                if cutscene_player.is_some() {
                    game_mode = GameMode::Cutscene {
                        kind: CutsceneKind::Ending(number),
                    };
                }
            }
        }

        // FUN_00488b90: while the game runs, the oldest story movie plays,
        // one at a time; a paused game keeps them waiting (FUN_0041dc10).
        // port: the strategic clock holds while it plays, as in every
        // cutscene; hyp: the original's keeps running.
        if game_mode == GameMode::Galaxy
            && !battle_underlay_pending
            && plays_story_movies
            && cutscene_player.is_none()
            && clock.speed != GameSpeed::Paused
            && !clock.pause_requested()
        {
            if let Some(movie) = msg_log.take_story_movie() {
                // FUN_00422ce0's 0x482: the music stops (FUN_00610c00(0)),
                // the galaxy view hides and the movie plays its caption.
                cutscene_player = open_cutscene(
                    Path::new(&story_cutscene_path(movie.movie)),
                    audio_vol.cutscene_volume(),
                    #[cfg(not(target_arch = "wasm32"))]
                    &mut audio_engine,
                    #[cfg(target_arch = "wasm32")]
                    browser_menu_audio.as_mut(),
                );
                if cutscene_player.is_some() {
                    macroquad::logging::info!(
                        "[cutscene] story movie={} caption={}",
                        movie.movie,
                        movie.caption
                    );
                    story_caption = Some(movie.caption);
                    game_mode = GameMode::Cutscene {
                        kind: CutsceneKind::Story(movie.movie),
                    };
                }
            }
        }

        // ── Rendering (mode-specific) ────────────────────────────────────────

        match game_mode {
            GameMode::Cutscene { ref kind } => {
                clear_background(BLACK);

                let next_mode = match kind {
                    CutsceneKind::Intro
                    | CutsceneKind::IntroCinematic
                    | CutsceneKind::Credits
                    | CutsceneKind::Ending(_) => GameMode::MainMenu,
                    CutsceneKind::NewGame => GameMode::GameSetup,
                    CutsceneKind::TrenchRun(_) => GameMode::TacticalCombat,
                    CutsceneKind::Story(_) => GameMode::Galaxy,
                };
                let chains_cinematic = *kind == CutsceneKind::Intro;
                let next_music = music_context_after_cutscene(kind);

                if let Some(player) = cutscene_player.as_mut() {
                    player.advance(dt);
                    if let Some(frame) = player.current_frame() {
                        draw_fullscreen_texture(frame);
                    }
                    if let Some(caption) = story_caption.as_deref() {
                        draw_story_caption(caption);
                    }
                    // FUN_005f6920 -> FUN_0060a9e0: MessageBeep(0x10) as the
                    // box opens. port: a synthesized stop beep stands in
                    // for the Windows sound (audio::stop_beep_wave).
                    if player.take_box_beep() {
                        #[cfg(not(target_arch = "wasm32"))]
                        let played = audio_engine.play_stop_beep(&audio_vol);
                        #[cfg(target_arch = "wasm32")]
                        let played = browser_menu_audio
                            .as_mut()
                            .is_some_and(|engine| engine.play_stop_beep(&audio_vol));
                        macroquad::logging::info!(
                            "[audio] movie_error_beep source=FUN_0060a9e0 message_beep=0x10 port=synthesized played={played} muted={}",
                            audio_vol.effective_sfx_volume() <= 0.0
                        );
                    }
                    answer_movie_error_box(player);

                    if player.is_finished() {
                        cutscene_player = None;
                        story_caption = None;
                        game_mode = next_mode;
                        // FUN_004138a0's 0x405: the logos' end (or skip)
                        // starts state 2, the opening cinematic.
                        if chains_cinematic {
                            cutscene_player = open_cutscene(
                                Path::new(INTRO_CINEMATIC),
                                audio_vol.cutscene_volume(),
                                #[cfg(not(target_arch = "wasm32"))]
                                &mut audio_engine,
                                #[cfg(target_arch = "wasm32")]
                                browser_menu_audio.as_mut(),
                            );
                            if cutscene_player.is_some() {
                                game_mode = GameMode::Cutscene {
                                    kind: CutsceneKind::IntroCinematic,
                                };
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(next_music) = next_music.filter(|_| cutscene_player.is_none()) {
                            start_context_score(
                                &mut audio_engine,
                                next_music,
                                &mut galaxy_score,
                                player_faction == MissionFaction::Alliance,
                                &sounds_dir,
                                &audio_vol,
                            );
                        }
                        // The movie halted the menu's score
                        // (`open_cutscene`), which starts again once the
                        // next screen shows (`score_may_start`); the galaxy
                        // view takes its next pick (`FUN_00422cb0`).
                        #[cfg(target_arch = "wasm32")]
                        if let Some(next_music) = next_music.filter(|_| cutscene_player.is_none()) {
                            start_context_score(
                                browser_menu_audio.as_mut(),
                                next_music,
                                &mut galaxy_score,
                                player_faction == MissionFaction::Alliance,
                                browser_main_theme.as_deref(),
                                &browser_screen_music,
                            );
                        }
                    }
                } else {
                    game_mode = next_mode;
                }
            }

            GameMode::MainMenu => {
                sector_window_state.clear();
                system_window_state.clear();
                fleet_window_state.clear();
                defenses_window_state.clear();
                assault_summary_state.clear();
                missions_window_state.clear();
                object_menu = None;
                targeting = None;
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_music_for_context(
                    MusicContext::MainMenu,
                    &sounds_dir,
                    &audio_vol,
                );
                // Outside a game `FUN_0041d3b0` gives MDATA.300: a galaxy
                // pick left from the game gives way to the menu's track.
                #[cfg(target_arch = "wasm32")]
                if browser_menu_audio
                    .as_ref()
                    .and_then(audio::AudioEngine::current_track)
                    .is_some_and(rebellion_render::MusicTrack::is_galaxy_pick)
                {
                    start_context_score(
                        browser_menu_audio.as_mut(),
                        MusicContext::MainMenu,
                        &mut galaxy_score,
                        player_faction == MissionFaction::Alliance,
                        browser_main_theme.as_deref(),
                        &browser_screen_music,
                    );
                }

                #[cfg(target_arch = "wasm32")]
                if let Some(focus) = web_accessibility::take_focus_update() {
                    main_menu_state.set_semantic_focus(focus);
                }
                #[cfg(target_arch = "wasm32")]
                let semantic_action = web_accessibility::take_activation()
                    .and_then(|control| main_menu_state.activate_control(control));
                #[cfg(target_arch = "wasm32")]
                let semantic_interaction = web_accessibility::take_user_interaction();

                clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
                #[cfg(target_arch = "wasm32")]
                let mut menu_action = semantic_action;
                #[cfg(not(target_arch = "wasm32"))]
                let mut menu_action = None;
                egui_macroquad::ui(|ctx| {
                    let canvas_action = draw_main_menu(
                        ctx,
                        &mut bmp_cache,
                        &mut main_menu_state,
                        audio_vol.music_enabled(),
                    );
                    if menu_action.is_none() {
                        menu_action = canvas_action;
                    }
                });
                egui_macroquad::draw();
                #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
                if menu_action.is_none() {
                    menu_action = scripted_menu_action.take();
                }

                if let Some(sfx) = main_menu_state.take_sfx() {
                    #[cfg(not(target_arch = "wasm32"))]
                    audio_engine.play_sfx(sfx, &audio_vol);
                    #[cfg(target_arch = "wasm32")]
                    if let Some(engine) = browser_menu_audio.as_mut() {
                        engine.play_sfx(sfx, &audio_vol);
                    }
                    let resource_id = audio::MENU_SFX_ASSETS
                        .iter()
                        .find_map(|&(kind, _, resource_id)| (kind == sfx).then_some(resource_id))
                        .unwrap_or_default();
                    macroquad::logging::info!(
                        "[audio] menu_sfx={:?} resource={}",
                        sfx,
                        resource_id
                    );
                }

                #[cfg(target_arch = "wasm32")]
                if !browser_menu_audio_requested
                    && (is_mouse_button_pressed(macroquad::input::MouseButton::Left)
                        || is_key_pressed(KeyCode::Enter)
                        || is_key_pressed(KeyCode::Space)
                        || is_key_pressed(KeyCode::Tab)
                        || semantic_interaction)
                {
                    browser_menu_audio_requested = true;
                    if browser_menu_audio.is_none() {
                        eprintln!(
                            "[audio] menu music unavailable: runtime pack has no MDATA.300 cue"
                        );
                    }
                }
                // The score starts after the frame's screen, at one place
                // (`score_may_start`), for the menu as for the galaxy view.

                if let Some(action) = menu_action {
                    match action {
                        MainMenuAction::StartGame {
                            difficulty,
                            faction,
                            galaxy_size,
                            headquarters_only,
                        } => {
                            // The original faction controls start immediately.
                            // Reuse the established campaign initialization path
                            // without displaying the replacement setup page.
                            game_setup_state.difficulty = difficulty;
                            game_setup_state.faction = Some(faction);
                            game_setup_state.galaxy_size = galaxy_size;
                            pending_victory_conditions = if headquarters_only {
                                VictoryConditions::HeadquartersOnly
                            } else {
                                VictoryConditions::Standard
                            };
                            pending_cockpit_start = Some(GameSetupAction::StartGame {
                                difficulty,
                                faction,
                                galaxy_size,
                            });
                            game_mode = GameMode::GameSetup;
                            // State 7 plays the side's movie before the game
                            // starts (state 8).
                            cutscene_player = open_cutscene(
                                Path::new(new_game_cutscene(faction == MissionFaction::Alliance)),
                                audio_vol.cutscene_volume(),
                                #[cfg(not(target_arch = "wasm32"))]
                                &mut audio_engine,
                                #[cfg(target_arch = "wasm32")]
                                browser_menu_audio.as_mut(),
                            );
                            if cutscene_player.is_some() {
                                game_mode = GameMode::Cutscene {
                                    kind: CutsceneKind::NewGame,
                                };
                            }
                        }
                        MainMenuAction::LoadGame => {
                            save_slots = read_save_slots(&saves_dir);
                            game_options_state.set_origin(GameOptionsOrigin::ShuttleCockpit);
                            game_options_state.refresh_saves(&save_slots);
                            game_mode = GameMode::GameOptions {
                                origin: GameOptionsOrigin::ShuttleCockpit,
                            };
                            macroquad::logging::info!(
                                "[interface] command=load_options destination=game_options status=opened_original"
                            );
                        }
                        // Command 0x73 (FUN_00406000): the credits are
                        // MDATA.005, after which the menu shows again.
                        MainMenuAction::Credits => {
                            cutscene_player = open_cutscene(
                                Path::new(CREDITS_CUTSCENE),
                                audio_vol.cutscene_volume(),
                                #[cfg(not(target_arch = "wasm32"))]
                                &mut audio_engine,
                                #[cfg(target_arch = "wasm32")]
                                browser_menu_audio.as_mut(),
                            );
                            if cutscene_player.is_some() {
                                game_mode = GameMode::Cutscene {
                                    kind: CutsceneKind::Credits,
                                };
                            }
                            macroquad::logging::info!("[main_menu] destination=credits");
                        }
                        MainMenuAction::Multiplayer => {
                            multiplayer_setup_state.status_message = None;
                            game_mode = GameMode::MultiplayerSetup;
                            macroquad::logging::info!("[main_menu] destination=multiplayer_setup");
                        }
                        MainMenuAction::ToggleMusic => {
                            audio_vol.toggle_music();
                            macroquad::logging::info!(
                                "[audio] menu_music_enabled={}",
                                audio_vol.music_enabled()
                            );
                        }
                        MainMenuAction::Quit => quit_requested = true,
                    }
                }
            }

            GameMode::MultiplayerSetup => {
                clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
                let mut multiplayer_action = None;
                egui_macroquad::ui(|ctx| {
                    multiplayer_action = draw_multiplayer_setup(ctx, &mut multiplayer_setup_state);
                });
                egui_macroquad::draw();
                match multiplayer_action {
                    Some(MultiplayerSetupAction::Back) => {
                        game_mode = GameMode::MainMenu;
                    }
                    Some(MultiplayerSetupAction::StartRequested) => {
                        let message = multiplayer_setup_state.unavailable_message();
                        multiplayer_setup_state.status_message = Some(message.clone());
                        macroquad::logging::info!(
                            "[multiplayer] status=unavailable transport={:?} player={}",
                            multiplayer_setup_state.transport,
                            multiplayer_setup_state.player_name
                        );
                    }
                    None => {}
                }
            }

            GameMode::GameOptions { origin } => {
                if let Some((slot, true)) = settle_game_options_save(
                    &mut game_options_state,
                    &mut save_slots,
                    &saves_dir,
                    rebellion_data::save::save_write_status(),
                ) {
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("Saved game to slot {}", slot + 1),
                        MessageCategory::Event,
                    ));
                }
                let mut action = draw_game_options(
                    &mut game_options_state,
                    &mut bmp_cache,
                    &save_slots,
                    &mut audio_vol,
                );
                egui_macroquad::ui(|ctx| {
                    let overlay = rebellion_render::game_options::draw_game_options_overlay(
                        ctx,
                        &mut bmp_cache,
                        &mut game_options_state,
                    );
                    if overlay != GameOptionsAction::None {
                        action = overlay;
                    }
                });
                egui_macroquad::draw();
                match action {
                    GameOptionsAction::None => {}
                    GameOptionsAction::Return => {
                        game_mode = match origin {
                            GameOptionsOrigin::ShuttleCockpit => GameMode::MainMenu,
                            GameOptionsOrigin::CommandCenter => GameMode::Galaxy,
                            GameOptionsOrigin::TacticalBattle => GameMode::TacticalCombat,
                        };
                        macroquad::logging::info!(
                            "[game_options] command=return origin={:?}",
                            origin
                        );
                    }
                    GameOptionsAction::Restart => {
                        game_mode = GameMode::MainMenu;
                        // Restart's 0x476 is FUN_00401040(3) (0x406fe5, jump
                        // table 0x4070cc): the cockpit is built anew
                        // (FUN_00414a80).
                        main_menu_state = main_menu_state.rebuilt();
                        macroquad::logging::info!("[game_options] command=restart");
                    }
                    GameOptionsAction::Exit => quit_requested = true,
                    GameOptionsAction::Save { slot, name } => {
                        panel_actions.push(PanelAction::SaveGame { slot, name });
                    }
                    GameOptionsAction::Load { slot } => {
                        panel_actions.push(PanelAction::LoadGame { slot });
                    }
                }
            }

            GameMode::GameSetup => {
                let mut setup_action = pending_cockpit_start.take();
                if setup_action.is_none() {
                    clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
                    egui_macroquad::ui(|ctx| {
                        setup_action = draw_game_setup(ctx, &mut game_setup_state);
                    });
                    egui_macroquad::draw();
                }

                if let Some(action) = setup_action {
                    match action {
                        GameSetupAction::StartGame {
                            difficulty,
                            faction,
                            galaxy_size,
                        } => {
                            player_faction = faction;

                            // Convert setup choices to SeedOptions and reload world.
                            let dat_faction_for_seed = match faction {
                                MissionFaction::Alliance => Faction::Alliance,
                                MissionFaction::Empire => Faction::Empire,
                            };
                            let seed_difficulty = match difficulty {
                                rebellion_render::Difficulty::Easy => SeedDifficulty::Easy,
                                rebellion_render::Difficulty::Medium => SeedDifficulty::Medium,
                                rebellion_render::Difficulty::Hard => SeedDifficulty::Hard,
                            };
                            let seed_options = SeedOptions {
                                galaxy_size,
                                difficulty: seed_difficulty,
                                player_faction: dat_faction_for_seed,
                                // Fresh random seed each game, unless
                                // OPEN_REBELLION_SEED fixes it.
                                #[cfg(not(target_arch = "wasm32"))]
                                rng_seed: dev_commands::campaign_seed(),
                                // The browser's default seed is the fixed 0
                                // its fixtures load with; a new game reads
                                // the clock (Date.now) instead.
                                #[cfg(target_arch = "wasm32")]
                                rng_seed: Some(
                                    rebellion_data::seeds::campaign_seed_from_clock(
                                        macroquad::miniquad::date::now(),
                                    ),
                                ),
                            };
                            campaign_config = CampaignConfig::from_seed_options(
                                seed_options,
                                pending_victory_conditions,
                            );
                            pending_victory_conditions = VictoryConditions::Standard;
                            let campaign_loaded = match rebellion_data::load_game_data_with_options(
                                &gdata_path,
                                &seed_options,
                            ) {
                                Ok(mut w) => {
                                    for error in mod_runtime.apply_enabled(&mut w) {
                                        macroquad::logging::error!(
                                            "[campaign] mod_reapply_error={:?}",
                                            error
                                        );
                                    }
                                    // The Fleet Registry's choice (a port extension).
                                    w.start_fleet_naming(main_menu_state.fleet_naming);
                                    macroquad::logging::info!(
                                        "[campaign] fleet_naming={:?}",
                                        main_menu_state.fleet_naming
                                    );
                                    world = w;
                                    drop_left_ending(&mut pending_ending);
                                    campaign_generation += 1;
                                    sim_rng = Xoshiro256PlusPlus::seed_from_u64(
                                        rng_seed.wrapping_add(u64::from(campaign_generation)),
                                    );
                                    clock = GameClock::new();
                                    ai_day = AiDay::default();
                                    day_rolls.clear();
                                    game_speed_ui = GameSpeedUiState::default();
                                    held_orders.clear();
                                    session_log.clear();
                                    session_needs_start = true;
                                    mfg_state = ManufacturingState::new();
                                    mission_state = MissionState::new();
                                    event_state = EventState::new();
                                    rebellion_core::story_events::define_story_events(
                                        &mut event_state,
                                        &world,
                                    );
                                    movement_state = MovementState::new();
                                    combat_cooldowns.clear();
                                    blockade_state = BlockadeState::new();
                                    uprising_state = UprisingState::new();
                                    research_state = ResearchState::new();
                                    repair_state = RepairState::default();
                                    production_state = ProductionState::new();
                                    autoscrap_state =
                                        rebellion_core::autoscrap::AutoscrapState::default();
                                    informant_state = rebellion_core::informant::InformantState::default();
                                    healing_state = rebellion_core::healing::HealingState::default();
                                    force_discovery_state =
                                        rebellion_core::force_discovery::ForceDiscoveryState::default();
                                    dagobah_state =
                                        rebellion_core::dagobah::DagobahState::default();
                                    final_battle_state =
                                        rebellion_core::final_battle::FinalBattleState::default();
                                    jabba_state = rebellion_core::jabba::JabbaState::default();
                                    seat_of_power_state = rebellion_core::seat_of_power::SeatOfPowerState::default();
                                    troop_transport_state = TroopTransportState::default();
                                    delivery_state = DeliveryState::new();
                                    economy_state = EconomyState::default();
                                    player_agent =
                                        PlayerAgent::for_new_game(campaign_config.difficulty);
                                    dual_ai_mode = false;
                                    secondary_ai_state = None;

                                    map_state = GalaxyMapState::default();
                                    warmed_galaxy_font_sizes.clear();
                                    msg_log = MessageLog::default();
                                    log_state = MessageLogState::default();
                                    build_selection_state = build_selection_state.for_next_game();
                                    mission_dialog_state = MissionDialogState::default();
                                    move_confirmation_state = MoveConfirmationState::default();
                                    status_window_state = StatusWindowState::default();
                                    fleet_finder_state = FleetFinderState::default();
                                    troop_finder_state = TroopFinderState::default();
                                    personnel_finder_state = PersonnelFinderState::default();
                                    system_finder_state = rebellion_render::system_finder::SystemFinderState::default();
                                    message_index = None;
                                    advice_saved_speed = None;
                                    enc_state = EncyclopediaState::new();
                                    enc_state.set_edata_path(configured_edata_path(&gdata_path));
                                    enc_state.set_asset_profile(asset_render_profile);
                                    enc_state.set_hd_path(
                                        gdata_path
                                            .parent()
                                            .unwrap_or(Path::new("."))
                                            .join("hd")
                                            .join("EData"),
                                    );
                                    save_slots = read_save_slots(&saves_dir);
                                    tactical_state = TacticalState::new();

                                    let alliance_hq = world
                                        .systems
                                        .iter()
                                        .find(|(_, system)| {
                                            system.is_headquarters
                                                && system
                                                    .control
                                                    .is_controlled_by(Faction::Alliance)
                                        })
                                        .map(|(key, _)| key);
                                    let empire_hq = world
                                        .systems
                                        .iter()
                                        .find(|(_, system)| {
                                            system.is_headquarters
                                                && system.control.is_controlled_by(Faction::Empire)
                                        })
                                        .map(|(key, _)| key);
                                    victory_state = if let (Some(alliance), Some(empire)) =
                                        (alliance_hq, empire_hq)
                                    {
                                        VictoryState::new(alliance, empire)
                                    } else {
                                        let mut keys = world.systems.keys();
                                        let alliance = keys.next().expect(
                                            "world must have at least 2 systems for victory",
                                        );
                                        let empire = keys.next().expect(
                                            "world must have at least 2 systems for victory",
                                        );
                                        VictoryState::new(alliance, empire)
                                    };
                                    true
                                }
                                Err(e) => {
                                    macroquad::logging::error!(
                                        "Failed to reload game data with seed options: {}",
                                        e
                                    );
                                    false
                                }
                            };

                            if campaign_loaded {
                                // Sync cockpit chrome to player faction
                                cockpit_state =
                                    CockpitState::new(if faction == MissionFaction::Alliance {
                                        CockpitFaction::Alliance
                                    } else {
                                        CockpitFaction::Empire
                                    });
                                sector_window_state.clear();
                                system_window_state.clear();
                                fleet_window_state.clear();
                                defenses_window_state.clear();
                                assault_summary_state.clear();
                                missions_window_state.clear();
                                object_menu = None;
                                targeting = None;

                                // Initialize game state for chosen faction
                                fog_alliance_state = FogState::new(Faction::Alliance);
                                fog_empire_state = FogState::new(Faction::Empire);
                                FogSystem::seed(&mut fog_alliance_state, &world);
                                FogSystem::seed(&mut fog_empire_state, &world);
                                // Each side's first views (FUN_004f9a60).
                                rebellion_core::object_views::seed_from_fog(
                                    &mut world,
                                    &troop_transport_state,
                                    &movement_state,
                                    [&fog_alliance_state, &fog_empire_state],
                                );
                                rebellion_core::object_views::seed_extras_from_fog(
                                    &mut world,
                                    &movement_state,
                                    &mission_state,
                                    [&fog_alliance_state, &fog_empire_state],
                                );
                                rebellion_core::object_views::seed_details_from_fog(
                                    &mut world,
                                    &movement_state,
                                    &mission_state,
                                    &troop_transport_state,
                                    [&fog_alliance_state, &fog_empire_state],
                                );
                                // The setup push (FUN_0051b990): a copy of
                                // every other system, and the Empire's
                                // level-10 push of Yavin, taken at once.
                                rebellion_core::object_views::start(
                                    &mut world,
                                    &movement_state,
                                    &mission_state,
                                    &troop_transport_state,
                                    &uprising_state,
                                );
                                economy_state = EconomyState::default();
                                // Setup stage 0x17: each system's
                                // FUN_00508250, once.
                                EconomySystem::setup(
                                    &mut economy_state,
                                    &world,
                                    world.difficulty_index,
                                    &uprising_state,
                                );
                                player_agent =
                                    PlayerAgent::for_new_game(campaign_config.difficulty);

                                ai_day = AiDay::default();
                                day_rolls.clear();
                                // AI controls the opposite faction
                                if faction == MissionFaction::Empire {
                                    ai_state = AIState::new(AiFaction::Alliance);
                                } else {
                                    ai_state = AIState::new(AiFaction::Empire);
                                }

                                let advisor_side = if faction == MissionFaction::Alliance {
                                    AdvisorFaction::Alliance
                                } else {
                                    AdvisorFaction::Empire
                                };
                                advisor_state.set_faction(advisor_side);
                                advisor_state.clips.clear();
                                briefing_caption = None;
                                advisor_schedule = new_advisor_schedule(&gdata_path, advisor_side);
                                // The galaxy view opens on Display Off; the
                                // briefing's action 1 or 0xb shows Popular
                                // Support (gid-markers.md, FUN_004c0fc0).
                                if advisor_schedule.is_some() {
                                    cockpit_state.gid_mode = rebellion_render::GidMode::DisplayOff;
                                }
                                let faction_name = if faction == MissionFaction::Alliance {
                                    "Rebel Alliance"
                                } else {
                                    "Galactic Empire"
                                };
                                let campaign_summary = campaign_config.summary();
                                macroquad::logging::info!(
                                    "[campaign] faction={} configuration={}",
                                    faction_name,
                                    campaign_summary
                                );
                                macroquad::logging::info!(
                                "[campaign] reset generation={} tick={} missions={} movements={} cooldowns={} dual_ai={} second_ai={} event_definitions={}",
                                campaign_generation,
                                clock.tick,
                                mission_state.len(),
                                movement_state.len(),
                                combat_cooldowns.len(),
                                dual_ai_mode,
                                secondary_ai_state.is_some(),
                                event_state.events().len()
                            );
                                msg_log.push(GameMessage::new(
                                    clock.tick,
                                    format!("You command the {faction_name} — {campaign_summary}."),
                                    MessageCategory::Event,
                                ));

                                // The galaxy window's creation starts its
                                // score (`FUN_00422ce0`: `FUN_00422cb0` ->
                                // `FUN_00417520(FUN_0041d3b0())`) with a
                                // new agent's counter and mood
                                // (`FUN_00439320`).
                                galaxy_score = rebellion_render::GalaxyScore::default();
                                galaxy_mood_day = None;
                                #[cfg(not(target_arch = "wasm32"))]
                                start_context_score(
                                    &mut audio_engine,
                                    MusicContext::GalaxyMap,
                                    &mut galaxy_score,
                                    player_faction == MissionFaction::Alliance,
                                    &sounds_dir,
                                    &audio_vol,
                                );
                                #[cfg(target_arch = "wasm32")]
                                start_context_score(
                                    browser_menu_audio.as_mut(),
                                    MusicContext::GalaxyMap,
                                    &mut galaxy_score,
                                    player_faction == MissionFaction::Alliance,
                                    browser_main_theme.as_deref(),
                                    &browser_screen_music,
                                );

                                // Sync advisor faction and send greeting
                                advisor_state =
                                    AdvisorState::new(AdvisorFaction::from(cockpit_state.faction));
                                let sprite_dir = gdata_path.join("ui");
                                advisor_state.set_sprite_dir(&sprite_dir);
                                advisor_greet(&mut advisor_state);

                                game_mode = GameMode::Galaxy;
                            } else {
                                game_mode = GameMode::MainMenu;
                            }
                        }
                        GameSetupAction::Back => {
                            game_mode = GameMode::MainMenu;
                        }
                    }
                }
            }

            GameMode::Galaxy => {
                prewarm_galaxy_font_sizes(&world, &map_state, &mut warmed_galaxy_font_sizes);
                let fog_state = if player_faction == MissionFaction::Alliance {
                    &fog_alliance_state
                } else {
                    &fog_empire_state
                };
                // 1. Prepare the original 640×480 command-center canvas and
                // recover the faction-specific galaxy aperture.
                // The authentic bitmap frame is drawn in the single egui pass
                // below so input is consumed exactly once per game frame.
                let cockpit_layout = draw_cockpit_chrome(&cockpit_state);
                let cockpit_vp = cockpit_layout.galaxy;

                // Pass cockpit viewport to galaxy map for mouse input clamping.
                map_state.viewport = Some((
                    cockpit_vp.x,
                    cockpit_vp.y,
                    cockpit_vp.width,
                    cockpit_vp.height,
                ));
                map_state.display_scale = cockpit_layout.scale;
                let pointer = mouse_position();
                #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                let original_modal_fixture_open =
                    interface_fixture_request.is_some_and(|request| {
                        matches!(
                            request.scenario,
                            interface_test_fixture::Scenario::EncyclopediaArtwork
                                | interface_test_fixture::Scenario::MessageIndexShell
                                | interface_test_fixture::Scenario::EncyclopediaIndexShell
                                | interface_test_fixture::Scenario::EncyclopediaIndexCatalog
                        )
                    });
                #[cfg(not(all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
                let original_modal_fixture_open = false;
                // An open Finder, Status window, Encyclopedia or Message
                // Index is modal: A0 (2026-10-30, `a0-vm/modal` and
                // `a0-vm/modal2`) takes no click on the cockpit controls, the
                // GID control, the rail, the droids or the map until it
                // closes.
                // The Battle Summary is one too (manual p. 64, "Modal vs.
                // Modaless Windows"): FUN_0046f140 passes 1 as FUN_00606380's
                // +0xb8, the flag FUN_00460090 sets on the System Finder.
                // FUN_00422ce0's WM_KEYDOWN drops F1..F7 (0x70..0x76) while
                // any child in the view's list has that bit; the gate below
                // holds the cockpit's F-keys and F6 with it.
                let window_modal_open = fleet_finder_state.is_open()
                    || troop_finder_state.is_open()
                    || personnel_finder_state.is_open()
                    || system_finder_state.is_open()
                    || status_window_state.is_open()
                    || enc_state.open
                    || message_index.is_some()
                    || tactical_state.battle_summary_open();
                // The "Resume Game Play?" alert holds the command center:
                // A0 speed2/pause-modal-* (no sector window closes, no rail
                // opens the index, no speed menu opens until its checkmark).
                let modal_window_open = window_modal_open || clock.pause_requested();
                map_state.pointer_blocked = modal_window_open
                    || sector_window_state
                    .contains_screen_point(cockpit_layout, pointer)
                    || system_window_state.contains_screen_point(cockpit_layout, pointer)
                    || fleet_window_state.contains_screen_point(cockpit_layout, pointer)
                    || defenses_window_state.contains_screen_point(cockpit_layout, pointer)
                    || missions_window_state.contains_screen_point(cockpit_layout, pointer)
                    || fleet_window_state.is_dragging()
                    || defenses_window_state.is_dragging()
                    || mission_dialog_state.contains_screen_point(cockpit_layout, pointer)
                    || build_selection_state.contains_screen_point(cockpit_layout, pointer)
                    || move_confirmation_state.contains_screen_point(cockpit_layout, pointer)
                    || status_window_state.contains_screen_point(cockpit_layout, pointer)
                    || assault_summary_state.contains_screen_point(cockpit_layout, pointer)
                    || fleet_finder_state.contains_screen_point(cockpit_layout, pointer)
                    || troop_finder_state.contains_screen_point(cockpit_layout, pointer)
                    || personnel_finder_state.contains_screen_point(cockpit_layout, pointer)
                    || message_index.is_some()
                        && rebellion_render::message_index::window_rect(cockpit_layout)
                            .contains(egui_macroquad::egui::pos2(pointer.0, pointer.1))
                    || cockpit_state.gid_ui.menu_open
                    || cockpit_state.gid_legend.contains_screen_point(
                        cockpit_layout,
                        cockpit_state.gid_mode,
                        cockpit_state.faction,
                        pointer,
                    )
                    || enc_state.open
                    || original_modal_fixture_open
                    || game_speed_ui.menu_anchor.is_some()
                    || object_menu.is_some()
                    || pause_alert_contains_screen_point(&clock, cockpit_layout, pointer)
                    || tactical_state
                        .battle_summary_contains_screen_point((screen_width(), screen_height()), pointer);
                map_state.targeting = targeting.is_some();

                // The starfield shows through every transparent part of the
                // shell, so it manages its own clip regions.
                draw_galaxy_backdrop(
                    cockpit_layout,
                    cockpit_state.faction,
                    &mut bmp_cache,
                    cockpit_state.gid_mode,
                );

                // Keep every other macroquad map layer inside the shell's
                // galaxy aperture. The clip is cleared before the egui pass.
                set_cockpit_viewport_clip(Some(cockpit_vp));

                // 2. Recovered GID baseline. Replacement fog, fleet, sector,
                // facility, and blockade primitives stay off the parity surface
                // until their original GID modes are reconstructed.
                // FUN_004c0fc0 selects the system 0x92 / 0x93 mark.
                let marked_system = map_state.selected_system;
                let galaxy_camera = draw_galaxy_map(
                    &world,
                    &mut map_state,
                    &mut bmp_cache,
                    cockpit_state.gid_mode,
                    cockpit_state.faction,
                    &rebellion_render::GidOverlayContext {
                        movement: &movement_state,
                        manufacturing: &mfg_state,
                        economy: &economy_state,
                        missions: &mission_state,
                        uprisings: &uprising_state,
                        marked_system,
                        fog: fog_state,
                    },
                );
                #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                if let Some(request) = interface_fixture_request {
                    if request.scenario == interface_test_fixture::Scenario::Hover {
                        interface_test_fixture::emit_hover(request, &world, &map_state);
                    }
                }
                if let Some(system) = map_state.activated_system {
                    // FUN_00422ce0: the click closes a visible legend first.
                    if cockpit_state.gid_legend.close_for_map_click() {
                        macroquad::logging::info!(
                            "[interface] command=0x405 destination=gid_legend status=closed_by_map_click"
                        );
                    }
                    sector_window_state.open_for_system(&world, system, cockpit_state.faction);
                }
                set_cockpit_viewport_clip(None);

                // The Message Index rail lights each category with unread
                // messages for the player's side (FUN_0048a2a0). The frame a
                // battle began in keeps the last frame's rail: the Battle
                // Alert's backdrop shows none of that day's messages lit
                // (A0 empire-game3/battle-alert-coruscant).
                if !battle_underlay_pending {
                    cockpit_state.message_unread_mask =
                        msg_log.unread_mask(player_faction == MissionFaction::Alliance);
                }

                // 4. All egui panels in a single ui() + draw() pass
                egui_macroquad::ui(|ctx| {
                    let strategic_input_enabled = !original_modal_fixture_open && !modal_window_open;
                    // The Battle Summary lies over every strategy window.
                    tactical_state.battle_summary_input_hold(ctx);
                    // The shared tooltip (FUN_006008e0), from the control
                    // the last frame registered, above every window.
                    {
                        let layout = cockpit_state.layout();
                        rebellion_render::win_tooltip::paint(
                            ctx,
                            egui_macroquad::egui::pos2(layout.canvas.x, layout.canvas.y),
                            layout.scale,
                        );
                    }
                    // Register the cockpit background before panels so the
                    // opaque chrome never covers their content or artwork.
                    draw_cockpit_background(ctx, &cockpit_state, &mut bmp_cache);
                    // The legend shows the galaxy view under it, shaded.
                    cockpit_state.gid_legend.set_view(
                        cockpit_state.gid_mode,
                        &map_state.drawn_gid_markers,
                        rebellion_render::gid_caption(
                            galaxy_camera,
                            cockpit_state.faction,
                            cockpit_state.gid_mode,
                            briefing_caption
                                .and_then(rebellion_render::advisor_schedule::caption_text),
                        ),
                    );
                    // Paint the native primary controls before floating
                    // windows. Input resolves after those windows register.
                    if let Some(mode) = draw_cockpit_egui_layer(
                        ctx,
                        &mut cockpit_state,
                        &mut bmp_cache,
                        strategic_input_enabled,
                    ) {
                        macroquad::logging::info!(
                            "[interface] command=0x{:x} destination=gid status=selected label={}",
                            mode.command_id(),
                            mode.label()
                        );
                        #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                        if let Some(request) = interface_fixture_request {
                            interface_test_fixture::emit_selected(request, cockpit_state.gid_mode);
                        }
                    }

                    rebellion_render::draw_sector_hover_label(ctx, &world, galaxy_camera, &map_state);
                    draw_gid_caption(
                        ctx,
                        galaxy_camera,
                        cockpit_state.faction,
                        cockpit_state.gid_mode,
                        briefing_caption
                            .and_then(rebellion_render::advisor_schedule::caption_text),
                    );
                    if briefing_holds_input {
                        rebellion_render::advisor_schedule::draw_input_hold(ctx);
                    }
                    // The day readout is the Game Speed control; a right
                    // click on it opens the original speed menu.
                    draw_day_readout(ctx, cockpit_layout, cockpit_state.faction, clock.tick);
                    draw_material_monitors(
                        ctx,
                        cockpit_layout,
                        cockpit_state.faction,
                        &production_state,
                    );
                    draw_maintenance_monitor(
                        ctx,
                        cockpit_layout,
                        cockpit_state.faction,
                        maintenance_available(
                            &world,
                            &mfg_state,
                            &delivery_state,
                            &blockade_state,
                            &uprising_state,
                            cockpit_state.faction,
                        ),
                    );
                    rebellion_render::resource_monitors::top_bar_tooltips(
                        ctx,
                        cockpit_layout,
                        cockpit_state.faction,
                        strategic_input_enabled,
                    );
                    draw_time_bar(
                        ctx,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        &clock,
                    );
                    let speed_input = strategic_input_enabled;
                    if speed_input {
                        open_game_speed_menu_on_right_click(
                            ctx,
                            &mut game_speed_ui,
                            cockpit_layout,
                            cockpit_state.faction,
                        );
                    }
                    if let Some(speed) = draw_game_speed_menu(
                        ctx,
                        &mut game_speed_ui,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        speed_input,
                    ) {
                        choose_game_speed(&mut clock, speed);
                    }
                    // A right click on the agent's droid opens the Agent
                    // menu (list 0xdead, FUN_00487900).
                    if speed_input
                        && rebellion_render::agent_menu::open_agent_menu_on_right_click(
                            ctx,
                            &mut agent_menu_state,
                            cockpit_layout,
                            cockpit_state.faction,
                        )
                    {
                        // FUN_0041d160 → FUN_00436640 → FUN_00439ed0: the
                        // droid answers the menu's opening.
                        if let Some(schedule) = advisor_schedule.as_mut() {
                            schedule.add_flags(rebellion_render::advisor_schedule::reaction::MENU);
                        }
                        macroquad::logging::info!("[advisor] reaction menu_opened count=1");
                    }
                    let agent_view = rebellion_render::agent_menu::AgentMenuView {
                        garrisons: player_agent.is_running(AutomationModule::Garrisons),
                        production: player_agent.is_running(AutomationModule::Production),
                        advice: player_agent.advice(),
                        translate: player_agent.translate(),
                    };
                    if let Some(command) = rebellion_render::agent_menu::draw_agent_menu(
                        ctx,
                        &mut agent_menu_state,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        agent_view,
                        speed_input,
                    ) {
                        apply_agent_command(
                            &mut player_agent,
                            &mut status_window_state,
                            &mut targeting,
                            command,
                            "menu",
                        );
                    }
                    // A right click on the second droid opens the Message
                    // Alerts menu (list 0xbeef, FUN_0041d190 → FUN_00487a60).
                    if speed_input
                        && rebellion_render::agent_menu::open_alerts_menu_on_right_click(
                            ctx,
                            &mut alerts_menu_state,
                            cockpit_layout,
                            cockpit_state.faction,
                        )
                    {
                        macroquad::logging::info!("[interface] message_alerts status=opened");
                    }
                    let player_alliance = player_faction == MissionFaction::Alliance;
                    let alerts_command = rebellion_render::agent_menu::draw_alerts_menu(
                        ctx,
                        &mut alerts_menu_state,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        |mask| {
                            msg_log.posting_option(Some(mask))
                                & rebellion_render::message_log::SILENT
                                != 0
                        },
                        speed_input,
                    );
                    match alerts_command {
                        Some(rebellion_render::agent_menu::AlertsCommand::ToggleAlert(mask)) => {
                            msg_log.toggle_silent(Some(mask));
                            macroquad::logging::info!(
                                "[interface] message_alerts mask={:#x} silent={}",
                                mask,
                                msg_log.posting_option(Some(mask))
                                    & rebellion_render::message_log::SILENT
                                    != 0
                            );
                        }
                        Some(rebellion_render::agent_menu::AlertsCommand::Messages)
                            if message_index.is_none() =>
                        {
                            // FUN_0041d770(1, 0x79): the index on All.
                            let mut index = MessageIndexState::new();
                            let actions = index.set_category(0x79);
                            message_index = Some(index);
                            apply_message_index_actions(
                                actions,
                                &mut msg_log,
                                &mut clock,
                                &mut advice_saved_speed,
                            );
                            macroquad::logging::info!(
                                "[interface] destination=message_index category=0x79 status=opened_by_alerts_menu side_alliance={}",
                                player_alliance
                            );
                        }
                        _ => {}
                    }
                    // A rail light's click (0x136..0x13e) or F6 (0x75) opens
                    // the Message Index on its category; FUN_0042a240 does
                    // nothing while one is open.
                    let index_command = speed_input
                        .then(|| {
                            if message_index_key {
                                return Some(0x75);
                            }
                            let (released, pointer) = ctx.input(|input| {
                                (
                                    input.pointer.primary_released(),
                                    input.pointer.interact_pos(),
                                )
                            });
                            let pointer =
                                pointer.filter(|_| released && !ctx.is_pointer_over_area())?;
                            rebellion_render::cockpit::message_index_command_at(
                                cockpit_state.faction,
                                (
                                    (pointer.x - cockpit_layout.canvas.x) / cockpit_layout.scale,
                                    (pointer.y - cockpit_layout.canvas.y) / cockpit_layout.scale,
                                ),
                            )
                        })
                        .flatten();
                    if let Some(command) = index_command {
                        if message_index.is_none() {
                            if let Some(category) =
                                rebellion_render::message_index::opening_category(command)
                            {
                                let mut index = MessageIndexState::new();
                                let actions = index.set_category(category);
                                message_index = Some(index);
                                apply_message_index_actions(
                                    actions,
                                    &mut msg_log,
                                    &mut clock,
                                    &mut advice_saved_speed,
                                );
                                macroquad::logging::info!(
                                    "[interface] command=0x{:x} destination=message_index category=0x{:x} status=opened_original",
                                    command,
                                    category
                                );
                            }
                        }
                    }
                    if let Some(index) = message_index.as_mut() {
                        let mut actions = rebellion_render::message_index::draw_message_index(
                            ctx,
                            &mut bmp_cache,
                            &mut enc_state,
                            cockpit_state.faction,
                            rebellion_render::message_index::window_rect(cockpit_layout).min,
                            cockpit_layout.scale,
                            &msg_log,
                            index,
                            player_faction == MissionFaction::Alliance,
                        );
                        // Display on a kind-4 message (FUN_00467f10 →
                        // FUN_0041d6b0): the Encyclopedia at its object.
                        for action in &actions {
                            if let MessageIndexAction::Encyclopedia(object) = *action {
                                match original_encyclopedia_catalog.as_ref() {
                                    Some(catalog) => rebellion_render::open_original_encyclopedia_at(
                                        &mut enc_state,
                                        catalog,
                                        Some(object),
                                    ),
                                    None => enc_state.open = true,
                                }
                                macroquad::logging::info!(
                                    "[interface] message_index encyclopedia object=0x{:08x} topic={:?} status=opened",
                                    object,
                                    enc_state.original_selected_object_id
                                );
                            }
                        }
                        // Display (0x67, FUN_00467f10 → FUN_00429440): the
                        // message's object; port: its system's sector window
                        // opens with the system chosen.
                        for action in &actions {
                            if let MessageIndexAction::Display { system, mission } = *action {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                                // A Mission Report's running mission: its
                                // Missions window (A0 mission-reports/report-
                                // display-missions-window-day235). port: an
                                // ended mission's report goes to its system.
                                let member = mission.and_then(|id| {
                                    mission_state
                                        .missions()
                                        .iter()
                                        .find(|value| value.id == id)
                                        .and_then(|value| value.members().next())
                                });
                                let mut shown = false;
                                if let (Some(member), Some(point)) = (
                                    member,
                                    sector_window_state.quadrant_window_point(
                                        &world,
                                        cockpit_layout,
                                        system,
                                        rebellion_render::quadrant_icons::Quadrant::Missions,
                                    ),
                                ) {
                                    missions_window_state.open(
                                        &world,
                                        fog_state,
                                        &mission_state,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                    shown = missions_window_state.show_member(
                                        &mission_state,
                                        system,
                                        member,
                                    );
                                }
                                macroquad::logging::info!(
                                    "[interface] message_index display system={:?} missions_window={}",
                                    world.systems.get(system).map(|value| value.name.as_str()),
                                    shown
                                );
                            }
                        }
                        // A Mission Report's X (Class_0065b710_vt3c): while
                        // the mission lasts, its held Abort goes out with
                        // force 1 (FUN_0041ce20 -> FUN_00487740), which
                        // validates (FUN_00521600: phase 8) and asks nothing;
                        // a refusal goes to the advisor (FUN_00487c90).
                        for action in &actions {
                            if let MessageIndexAction::AbortMission { mission, .. } = *action {
                                let allowed = mission_state.abort_allowed(mission, player_faction);
                                let aborted = allowed && mission_state.abort(mission);
                                macroquad::logging::info!(
                                    "[interface] message_index answer=abort mission={} result={}",
                                    mission,
                                    aborted
                                );
                            }
                            if let MessageIndexAction::ContinueMission { message } = *action {
                                macroquad::logging::info!(
                                    "[interface] message_index answer=continue message={}",
                                    message
                                );
                            }
                        }
                        // FUN_00469de0: a battle report (type 0x0f) opens
                        // its Assault Summary (FUN_0042a410) and the index
                        // closes (slot +0x30).
                        let report = actions.iter().find_map(|action| match action {
                            MessageIndexAction::MessageDisplayed(id) => msg_log
                                .messages()
                                .iter()
                                .find(|message| message.id == *id)
                                .and_then(|message| {
                                    message.assault_report.clone().map(|report| (*id, report))
                                }),
                            _ => None,
                        });
                        if let Some((id, report)) = report {
                            assault_summary_state.open(&world, *report, &message_texts, Some(id));
                            actions.push(MessageIndexAction::Close);
                        }
                        if actions.contains(&MessageIndexAction::Close) {
                            let closing = message_index.take().map(|index| index.close());
                            apply_message_index_actions(
                                actions
                                    .into_iter()
                                    .chain(closing.into_iter().flatten())
                                    .collect(),
                                &mut msg_log,
                                &mut clock,
                                &mut advice_saved_speed,
                            );
                        } else {
                            apply_message_index_actions(
                                actions,
                                &mut msg_log,
                                &mut clock,
                                &mut advice_saved_speed,
                            );
                        }
                    }
                    // FUN_0048a610 with the Message Index's state, then
                    // FUN_0048a2a0 once a message was filed. Facility Idle
                    // (FUN_0048c2b0) lasts while the system is the player's,
                    // its area builds nothing and still has its facility.
                    {
                        let player_side = if player_faction == MissionFaction::Alliance {
                            Faction::Alliance
                        } else {
                            Faction::Empire
                        };
                        let idle = |system, area| {
                            world.systems.get(system).is_some_and(|value| {
                                value.control.faction() == Some(player_side)
                            }) && mfg_state
                                .queue(system, area)
                                .is_none_or(|queue| queue.items().is_empty())
                                && pictures::idle_yard(&world, system, area).is_some()
                        };
                        msg_log.set_index_open(message_index.is_some(), &idle);
                        msg_log.expire_if_due(&idle);
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_story_messages(
                            request,
                            &msg_log,
                            message_index.as_ref(),
                            player_faction == MissionFaction::Alliance,
                        );
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        speed_menu_watch.observe(
                            request,
                            game_speed_ui.menu_anchor.and_then(|_| {
                                rebellion_render::game_speed::game_speed_menu_rect(ctx)
                            }),
                        );
                    }
                    let pause_input = !original_modal_fixture_open && !window_modal_open;
                    if draw_pause_alert(ctx, &clock, &mut bmp_cache, cockpit_layout, pause_input) {
                        clock.resume();
                    }

                    // The original Galactic Encyclopedia (FUN_00429f30,
                    // FUN_0045d400) when its source loaded, else the
                    // replacement window.
                    let original_topics = original_encyclopedia_topics
                        [usize::from(cockpit_state.faction == CockpitFaction::Empire)]
                    .as_ref();
                    // FUN_00422620 lists the systems of the viewer's side
                    // (FUN_004f31b0): those of this game's galaxy.
                    if let (true, Some(catalog)) =
                        (enc_state.open, original_encyclopedia_catalog.as_mut())
                    {
                        catalog.set_galaxy_systems(
                            world.systems.values().map(|system| system.dat_id.raw()),
                        );
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let (Some(request), Some(catalog)) =
                        (interface_fixture_request, original_encyclopedia_catalog.as_ref())
                    {
                        interface_test_fixture::emit_encyclopedia(
                            request,
                            &enc_state,
                            catalog,
                            original_topics,
                        );
                    }
                    if let (true, Some(catalog), Some(topics)) = (
                        enc_state.open,
                        original_encyclopedia_catalog.as_ref(),
                        original_topics,
                    ) {
                        // Centred in the galaxy view (A0 2026-10-29).
                        let origin = rebellion_render::encyclopedia_origin(cockpit_layout);
                        let selected_topic = enc_state
                            .original_selected_object_id
                            .and_then(|object| topics.get(&object))
                            .cloned();
                        let command = match (enc_state.original_topic_mode, selected_topic) {
                            (true, Some(topic)) => rebellion_render::draw_encyclopedia_topic(
                                ctx,
                                &mut bmp_cache,
                                cockpit_state.faction,
                                origin,
                                cockpit_layout.scale,
                                &mut enc_state,
                                catalog,
                                &topic,
                            ),
                            _ => {
                                enc_state.original_topic_mode = false;
                                rebellion_render::draw_encyclopedia_index_catalog(
                                    ctx,
                                    &mut bmp_cache,
                                    cockpit_state.faction,
                                    origin,
                                    cockpit_layout.scale,
                                    &mut enc_state,
                                    catalog,
                                )
                            }
                        };
                        if let Some(command) = command {
                            macroquad::logging::info!(
                                "[encyclopedia] command={command:#x} topic_mode={} object={:?}",
                                enc_state.original_topic_mode,
                                enc_state.original_selected_object_id,
                            );
                        }
                        // The rail's close (0xfb).
                        if command == Some(0xfb) {
                            enc_state.open = false;
                            enc_state.original_topic_mode = false;
                            macroquad::logging::info!(
                                "[interface] command=0xfb destination=encyclopedia status=closed"
                            );
                        }
                    } else if let Some(sys_key) =
                        draw_encyclopedia(ctx, &world, &mut enc_state, &mut bmp_cache)
                    {
                        panel_actions.push(PanelAction::FocusFleetSystem(sys_key));
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if interface_fixture_request.is_some_and(|request| {
                        request.scenario == interface_test_fixture::Scenario::EncyclopediaArtwork
                    }) {
                        rebellion_render::draw_encyclopedia_artwork_fixture(
                            ctx,
                            42,
                            &mut enc_state,
                        );
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request.filter(|request| {
                        request.scenario == interface_test_fixture::Scenario::MessageIndexShell
                    }) {
                        rebellion_render::draw_message_index_fixture(
                            ctx,
                            &mut bmp_cache,
                            request.faction,
                        );
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request.filter(|request| {
                        request.scenario == interface_test_fixture::Scenario::EncyclopediaIndexShell
                    }) {
                        if let Some(command) = rebellion_render::draw_encyclopedia_index_fixture(
                            ctx,
                            &mut bmp_cache,
                            request.faction,
                            enc_state.original_category_command,
                        ) {
                            if (0x6f..=0x75).contains(&command) {
                                enc_state.original_category_command = command;
                            }
                        }
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let (Some(request), Some(catalog)) = (
                        interface_fixture_request.filter(|request| {
                            request.scenario
                                == interface_test_fixture::Scenario::EncyclopediaIndexCatalog
                        }),
                        original_encyclopedia_catalog.as_ref(),
                    ) {
                        rebellion_render::draw_encyclopedia_index_catalog_fixture(
                            ctx,
                            &mut bmp_cache,
                            request.faction,
                            &mut enc_state,
                            catalog,
                        );
                    }

                    // Mod Manager (floating window)
                    let mod_infos: Vec<rebellion_render::ModInfo> = mod_runtime
                        .discovered
                        .iter()
                        .map(|m| {
                            let err = mod_runtime.errors.iter().find(|e| e.mod_name() == m.name);
                            rebellion_render::ModInfo {
                                name: m.name.clone(),
                                version: m.version.clone(),
                                author: m.author.clone(),
                                description: m.description.clone(),
                                enabled: m.enabled,
                                dependencies: m.dependencies.keys().cloned().collect(),
                                has_error: err.is_some(),
                                error_message: err.map(ToString::to_string),
                            }
                        })
                        .collect();
                    let mod_actions =
                        rebellion_render::draw_mod_manager(ctx, &mod_infos, &mut mod_manager_state);
                    for action in mod_actions {
                        match action {
                            rebellion_render::ModManagerAction::ToggleMod(name) => {
                                panel_actions.push(PanelAction::ToggleMod { name });
                            }
                            rebellion_render::ModManagerAction::ReloadMods => {
                                panel_actions.push(PanelAction::ReloadMods);
                            }
                        }
                    }

                    // Developer command palette: its commands take the
                    // paths the clicks they replace would.
                    #[cfg(not(any(debug_assertions, not(target_arch = "wasm32"))))]
                    let commanded_sector_actions = Vec::new();
                    #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
                    let commanded_sector_actions = {
                        let mut commanded = Vec::new();
                        if palette_enabled {
                            for action in rebellion_render::draw_command_palette(
                                ctx,
                                &mut command_palette_state,
                            ) {
                                match action {
                                    rebellion_render::PaletteAction::Panel(action) => {
                                        panel_actions.push(action);
                                    }
                                    rebellion_render::PaletteAction::Interface(command) => {
                                        pending_interface.push(command);
                                    }
                                }
                            }
                        }
                        for command in std::mem::take(&mut pending_interface) {
                            match sector_window_state.command_actions(
                                &world,
                                fog_state,
                                &mission_state,
                                &movement_state,
                                rebellion_render::quadrant_icons::Travellers {
                                    regiments: troop_transport_state.transits(),
                                    squadrons: troop_transport_state.squadrons.transits(),
                                },
                                cockpit_state.faction,
                                cockpit_layout,
                                command,
                            ) {
                                Ok(actions) => commanded.extend(actions),
                                Err(why) => eprintln!("[dev-command] refused: {why}"),
                            }
                        }
                        commanded
                    };

                    sector_window_state.set_backdrop(cockpit_state.gid_mode);
                    sector_window_state.set_gid_markers(&map_state.gid_markers);
                    sector_window_state.set_drawn_gid_markers(&map_state.drawn_gid_markers);
                    sector_window_state.set_gid_caption(rebellion_render::gid_caption(
                        galaxy_camera,
                        cockpit_state.faction,
                        cockpit_state.gid_mode,
                        briefing_caption.and_then(rebellion_render::advisor_schedule::caption_text),
                    ));
                    for action in draw_sector_windows(
                        ctx,
                        &world,
                        &movement_state,
                        rebellion_render::quadrant_icons::Travellers {
                            regiments: troop_transport_state.transits(),
                            squadrons: troop_transport_state.squadrons.transits(),
                        },
                        fog_state,
                        &mut sector_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                        &uprising_state,
                        &mission_state,
                    )
                    .into_iter()
                    .chain(commanded_sector_actions)
                    {
                        match action {
                            SectorWindowAction::SelectSystem(system) => {
                                map_state.selected_system = Some(system);
                            }
                            SectorWindowAction::OpenSystemWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
                                system_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SectorWindowAction::OpenFleetWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
                                // A railed window comes back from the rail
                                // (A0 personnel-states/38, the Defenses').
                                let railed = system_window_state.take_railed_fleet(system);
                                fleet_window_state.open(
                                    &world,
                                    system,
                                    railed.unwrap_or(logical_position),
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SectorWindowAction::OpenDefensesWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
                                // A railed window comes back where it stood,
                                // on its page (A0 personnel-states/38).
                                let railed = system_window_state.take_railed_defenses(system);
                                defenses_window_state.open(
                                    &world,
                                    system,
                                    railed.map_or(logical_position, |(at, _)| at),
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                                if let Some((_, page)) = railed {
                                    defenses_window_state.show_page(system, page);
                                }
                            }
                            SectorWindowAction::OpenMissionsWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
                                // A railed window comes back from the rail
                                // (A0 personnel-states/38, the Defenses').
                                let railed = system_window_state.take_railed_missions(system);
                                missions_window_state.open(
                                    &world,
                                    fog_state,
                                    &mission_state,
                                    system,
                                    railed.unwrap_or(logical_position),
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SectorWindowAction::OpenObjectMenu { selection, point } => {
                                let mut gates = order_gates(
                                    selection,
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &delivery_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                    victory_state.conditions_word,
                                );
                                gates.abort = !abort_missions(
                                    &world,
                                    fog_state,
                                    &mission_state,
                                    selection,
                                    player_faction,
                                )
                                .is_empty();
                                object_menu = Some(ObjectMenuState::new(selection, gates, point));
                            }
                            // FUN_00422ce0, window type 1: the selection moves
                            // against what +0x70 gives under the point, kind
                            // 4 entries apart (0x214 against +0x68, below).
                            // The fleet icon's fleets are the system's of the
                            // player's side (FUN_00512700).
                            SectorWindowAction::DropSelection {
                                selection:
                                    MenuObject::SystemIcon {
                                        system,
                                        quadrant: Quadrant::Fleets,
                                    },
                                point,
                                confirmed,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                if let Some(target) = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    let fleets = system_side_fleets(
                                        &movement_state,
                                        &world,
                                        system,
                                        player_faction == MissionFaction::Alliance,
                                    );
                                    issue_fleet_move(
                                        &FleetMoveContext {
                                            world: &world,
                                            movement: &movement_state,
                                            blockaded: blockade_state.blockaded_systems(),
                                            faction: player_faction,
                                            tick: clock.tick,
                                            icon: true,
                                        },
                                        &fleets,
                                        confirmed,
                                        target,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    );
                                }
                            }
                            // FUN_00422ce0, window type 1: a kind 4 entry is
                            // split off into a 0x214 order (Destination)
                            // against +0x68; its team is the system's
                            // managers of the player's (FUN_00512700 kind
                            // 4). FUN_00487cc0 asks nothing for 0x214.
                            SectorWindowAction::DropSelection {
                                selection:
                                    MenuObject::SystemIcon {
                                        system,
                                        quadrant: Quadrant::System,
                                    },
                                point,
                                ..
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let side = Faction::from(player_faction);
                                let sources = MissionReleaseSources {
                                    world: &world,
                                    movement: &movement_state,
                                    fog: fog_state,
                                    missions: &mission_state,
                                    manufacturing: &mfg_state,
                                    deliveries: &delivery_state,
                                    transport: &troop_transport_state,
                                };
                                let team = facility_icon_has_managers(&world, system, side);
                                let destination = if team {
                                    destination_drop_target(
                                        ctx,
                                        sources,
                                        cockpit_layout,
                                        windows,
                                        &mut bmp_cache,
                                        point,
                                    )
                                } else {
                                    None
                                };
                                match destination {
                                    Some(destination) => {
                                        panel_actions.push(PanelAction::SetDestination {
                                            system,
                                            area: None,
                                            destination,
                                        });
                                    }
                                    None => {
                                        macroquad::logging::info!(
                                            "[interface] command=0x214 destination=production_destination status=refused drop=1"
                                        );
                                    }
                                }
                            }
                            // FUN_00422ce0, window type 1: a kind 8 entry
                            // moves against +0x70, a Confirmed Move with
                            // Ctrl; its team is the system's regiments,
                            // squadrons, characters and special forces of
                            // the player's (FUN_00512700 kind 8).
                            SectorWindowAction::DropSelection {
                                selection:
                                    MenuObject::SystemIcon {
                                        system,
                                        quadrant: Quadrant::Defenses,
                                    },
                                point,
                                confirmed,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                if let Some(target) = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    issue_defenses_move(
                                        &DefensesMoveContext {
                                            world: &world,
                                            movement: &movement_state,
                                            missions: &mission_state,
                                            transport: &troop_transport_state,
                                            deliveries: &delivery_state,
                                            blockaded: blockade_state.blockaded_systems(),
                                            faction: player_faction,
                                            tick: clock.tick,
                                        },
                                        system,
                                        confirmed,
                                        target,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    );
                                }
                            }
                            // FUN_00422ce0, window type 1: a kind 0x40 entry
                            // moves against +0x70 too. Kind 0x40 ignores the
                            // order's kind (FUN_00512700), so the team is the
                            // members on visible missions; the 0x241's
                            // command validator (Event250_vt1c FUN_0054d280
                            // -> FUN_00542b60 -> FUN_0054c200 ->
                            // FUN_00522b30) refuses a member that has a
                            // mission, and an empty team is 1/0x16
                            // (FUN_0053c4b0). Every such drop is refused.
                            SectorWindowAction::DropSelection {
                                selection:
                                    MenuObject::SystemIcon {
                                        quadrant: Quadrant::Missions,
                                        ..
                                    },
                                point,
                                confirmed,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                if release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                )
                                .is_some()
                                {
                                    macroquad::logging::info!(
                                        "[interface] command={:#x} missions_icon status=refused",
                                        if confirmed { 0x202 } else { 0x201 }
                                    );
                                    // port: FUN_00487c90 plays the side's
                                    // advisor reaction; this line stands in.
                                    msg_log.refuse(
                                        GameMessage::new(
                                            clock.tick,
                                            "Move refused".to_string(),
                                            MessageCategory::Event,
                                        ),
                                        UNTRACED_REFUSAL,
                                    );
                                }
                            }
                            // The manual (p. 134): "drag the HQ icon onto a
                            // new system"; Ctrl makes it a Confirmed Move.
                            SectorWindowAction::DropSelection {
                                selection: MenuObject::Headquarters(_),
                                point,
                                confirmed,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                if let Some(ReleaseTarget::System(destination)) = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    order_headquarters_move(
                                        &world,
                                        &troop_transport_state,
                                        player_faction,
                                        destination,
                                        confirmed,
                                        clock.tick,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    );
                                }
                            }
                            SectorWindowAction::DropSelection { .. } => {}
                        }
                    }

                    // FUN_00457690: only the galaxy view's focused child
                    // draws the focused strip, across window kinds.
                    let [system_focus, fleet_focus, defenses_focus, missions_focus] =
                        rebellion_render::window_focus::shared_focus([
                            system_window_state.focus_stamp(),
                            fleet_window_state.focus_stamp(),
                            defenses_window_state.focus_stamp(),
                            missions_window_state.focus_stamp(),
                        ]);
                    system_window_state.set_holds_focus(system_focus);
                    fleet_window_state.set_holds_focus(fleet_focus);
                    defenses_window_state.set_holds_focus(defenses_focus);
                    missions_window_state.set_holds_focus(missions_focus);

                    for action in draw_system_windows(
                        ctx,
                        &world,
                        &movement_state,
                        fog_state,
                        &mission_state,
                        &mfg_state,
                        &delivery_state,
                        &mut system_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        match action {
                            SystemWindowAction::FocusSector(system) => {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                            }
                            SystemWindowAction::SelectSystem(system) => {
                                map_state.selected_system = Some(system);
                            }
                            SystemWindowAction::RestoreFleetWindow {
                                system,
                                logical_position,
                            } => {
                                fleet_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SystemWindowAction::RestoreDefensesWindow {
                                system,
                                logical_position,
                                page,
                            } => {
                                defenses_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                                defenses_window_state.show_page(system, page);
                            }
                            SystemWindowAction::RestoreMissionsWindow {
                                system,
                                logical_position,
                            } => {
                                missions_window_state.open(
                                    &world,
                                    fog_state,
                                    &mission_state,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SystemWindowAction::OpenObjectMenu {
                                selection, point, ..
                            } => {
                                let gates = order_gates(
                                    selection,
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &delivery_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                    victory_state.conditions_word,
                                );
                                object_menu = Some(ObjectMenuState::new(selection, gates, point));
                            }
                            // FUN_00422ce0's 0x29a, window type 9: a
                            // Destination (0x214) against the +0x70 object
                            // under the drop, for the dragged band's manager
                            // (A0 2026-11-21-build-dest-16/28..33).
                            SystemWindowAction::DropBand {
                                system,
                                area,
                                point,
                            } => {
                                let pointer = egui_macroquad::egui::pos2(
                                    cockpit_layout.canvas.x
                                        + f32::from(point.0) * cockpit_layout.scale,
                                    cockpit_layout.canvas.y
                                        + f32::from(point.1) * cockpit_layout.scale,
                                );
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let target = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    pointer,
                                );
                                if let Some(action) = band_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    pointer,
                                    (system, area),
                                    target,
                                    player_faction == MissionFaction::Alliance,
                                ) {
                                    panel_actions.push(action);
                                }
                            }
                        }
                    }

                    fleet_window_state
                        .set_arrivals(rebellion_render::quadrant_icons::fleet_arrivals(&mission_state));
                    for action in draw_fleet_windows(
                        ctx,
                        &world,
                        &movement_state,
                        fog_state,
                        &troop_transport_state,
                        &mut fleet_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        match action {
                            FleetWindowAction::OpenSector(system) => {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                            }
                            FleetWindowAction::SelectSystem(system) => {
                                map_state.selected_system = Some(system);
                            }
                            FleetWindowAction::Minimize {
                                system,
                                logical_position,
                            } => {
                                system_window_state.minimize_fleet_window(system, logical_position)
                            }
                            // FUN_00422ce0: a drop from the Fleet window
                            // (type 4) moves the selection against the
                            // window under the point with 0x201.
                            FleetWindowAction::Rename {
                                entry:
                                    rebellion_render::fleet_window::FleetWindowEntry::Product {
                                        product,
                                        ..
                                    },
                                name,
                            } => {
                                panel_actions.push(PanelAction::RenameProduct { product, name });
                            }
                            FleetWindowAction::Rename { entry, name } => {
                                panel_actions.push(PanelAction::Rename {
                                    fleet: entry.fleet(),
                                    ship: match entry {
                                        rebellion_render::fleet_window::FleetWindowEntry::Ship { index, .. } => Some(index),
                                        rebellion_render::fleet_window::FleetWindowEntry::Fleet(_)
                                        | rebellion_render::fleet_window::FleetWindowEntry::Product { .. }
                                        | rebellion_render::fleet_window::FleetWindowEntry::Copy { .. } => None,
                                    },
                                    name,
                                });
                            }
                            FleetWindowAction::OpenObjectMenu {
                                system,
                                selection,
                                team,
                                point,
                            } => {
                                let personnel = |members: &[MissionMember]| {
                                    mission_state.move_order_enabled(
                                        &world,
                                        player_faction,
                                        members,
                                    )
                                };
                                let gates = team_gates(
                                    &team,
                                    |object| {
                                        order_gates(
                                            Some(object),
                                            &world,
                                            &mission_state,
                                            &movement_state,
                                            &troop_transport_state,
                                            &delivery_state,
                                            &mfg_state,
                                            &research_state,
                                            player_faction,
                                            victory_state.conditions_word,
                                        )
                                    },
                                    Some(&personnel),
                                );
                                object_menu_place = Some((system, team_squadron_place(&team, system)));
                                object_menu = Some(
                                    ObjectMenuState::new(Some(selection), gates, point)
                                        .with_team(team),
                                );
                            }
                            // FUN_00422ce0, window type 4: the selection's
                            // 0x201 (0x202 with Ctrl) against what +0x70
                            // gives under the point, as one order.
                            action @ (FleetWindowAction::DragFleet { .. }
                            | FleetWindowAction::DragShip { .. }
                            | FleetWindowAction::DragProducts { .. }
                            | FleetWindowAction::DragRegiment { .. }
                            | FleetWindowAction::DragMembers { .. }
                            | FleetWindowAction::DragSquadron { .. }) => {
                                let Some((order, point, confirmed)) =
                                    fleet_drop_order(&world, action)
                                else {
                                    continue;
                                };
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let Some(target) = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) else {
                                    continue;
                                };
                                match order {
                                    FleetDropOrder::Fleets(fleets) => issue_fleet_move(
                                        &FleetMoveContext {
                                            world: &world,
                                            movement: &movement_state,
                                            blockaded: blockade_state.blockaded_systems(),
                                            faction: player_faction,
                                            tick: clock.tick,
                                            icon: false,
                                        },
                                        &fleets,
                                        confirmed,
                                        target,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    ),
                                    FleetDropOrder::Ships(team) if confirmed => confirm_ship_move(
                                        &world,
                                        player_faction,
                                        team,
                                        target,
                                        &mut move_confirmation_state,
                                    ),
                                    FleetDropOrder::Ships(team) => {
                                        panel_actions.push(ship_move_action(team, target));
                                    }
                                    // An unfinished unit's drop: its Move
                                    // (FUN_00487cc0 asks for 0x202, and
                                    // for 0x201 out of its side's
                                    // blockaded system).
                                    FleetDropOrder::Products(products) => {
                                        let (system, fleet) = match target {
                                            ReleaseTarget::System(system) => (system, None),
                                            ReleaseTarget::Fleet { fleet, system } => {
                                                (system, Some(fleet))
                                            }
                                        };
                                        let blockaded =
                                            rebellion_core::products::move_asks(&world, &products);
                                        if confirmed || blockaded {
                                            move_confirmation_state.open(product_confirmation(
                                                &world,
                                                player_faction,
                                                products,
                                                (system, fleet),
                                                blockaded,
                                            ));
                                        } else {
                                            panel_actions.push(PanelAction::MoveProducts {
                                                products,
                                                system,
                                                fleet,
                                            });
                                        }
                                    }
                                    FleetDropOrder::Team { place, team } => issue_team_move(
                                        &DefensesMoveContext {
                                            world: &world,
                                            movement: &movement_state,
                                            missions: &mission_state,
                                            transport: &troop_transport_state,
                                            deliveries: &delivery_state,
                                            blockaded: blockade_state.blockaded_systems(),
                                            faction: player_faction,
                                            tick: clock.tick,
                                        },
                                        &team,
                                        place.unwrap_or_else(|| {
                                            let system = target.system();
                                            (system, SquadronPlace::Planet(system))
                                        }),
                                        false,
                                        confirmed,
                                        target,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    ),
                                }
                            }
                        }
                    }

                    // The Assault Summary windows (FUN_0044c410).
                    for action in rebellion_render::assault_summary::draw_assault_summaries(
                        ctx,
                        &mut assault_summary_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        use rebellion_render::assault_summary::AssaultSummaryAction;
                        match action {
                            // FUN_00469d60: the message is marked.
                            AssaultSummaryAction::Closed(message) => {
                                if let Some(id) = message {
                                    msg_log.mark_message_read(id);
                                }
                            }
                            // 0xb: FUN_00429ce0 opens the system's sector
                            // window; 0xc also its Fleet window
                            // (FUN_0045c8e0, kind 4).
                            AssaultSummaryAction::GotoSystem(system)
                            | AssaultSummaryAction::GotoFleets(system) => {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                                if matches!(action, AssaultSummaryAction::GotoFleets(_)) {
                                    let _ = fleet_window_state.open(
                                        &world,
                                        system,
                                        (85, 55),
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                }
                            }
                        }
                    }

                    for action in draw_defenses_windows(
                        ctx,
                        &world,
                        fog_state,
                        &mission_state,
                        rebellion_render::DefensesProduction {
                            manufacturing: &mfg_state,
                            deliveries: &delivery_state,
                            regiments: troop_transport_state.transits(),
                            squadrons: troop_transport_state.squadrons.transits(),
                        },
                        &economy_state,
                        &mut defenses_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        match action {
                            DefensesWindowAction::OpenSector(system) => {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                            }
                            DefensesWindowAction::SelectSystem(system) => {
                                map_state.selected_system = Some(system);
                            }
                            DefensesWindowAction::Minimize {
                                system,
                                logical_position,
                                page,
                            } => system_window_state
                                .minimize_defenses_window(system, logical_position, page),
                            DefensesWindowAction::OpenObjectMenu {
                                system,
                                selection,
                                team,
                                point,
                            } => {
                                let gates_of = |object| {
                                    order_gates(
                                        object,
                                        &world,
                                        &mission_state,
                                        &movement_state,
                                        &troop_transport_state,
                                        &delivery_state,
                                        &mfg_state,
                                        &research_state,
                                        player_faction,
                                        victory_state.conditions_word,
                                    )
                                };
                                let gates = if team.is_empty() {
                                    gates_of(selection)
                                } else {
                                    let personnel = |members: &[MissionMember]| {
                                        mission_state.move_order_enabled(
                                            &world,
                                            player_faction,
                                            members,
                                        )
                                    };
                                    team_gates(
                                        &team,
                                        |object| gates_of(Some(object)),
                                        Some(&personnel),
                                    )
                                };
                                object_menu_place = Some((system, team_squadron_place(&team, system)));
                                object_menu = Some(
                                    ObjectMenuState::new(selection, gates, point).with_team(team),
                                );
                            }
                            // FUN_00422ce0, window type 10: the selection's
                            // 0x201 (0x202 with Ctrl) against the window
                            // under the point, split as a defenses icon's
                            // (FUN_0053c4b0).
                            DefensesWindowAction::DragSelection {
                                system,
                                items,
                                point,
                                confirmed,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let Some(target) = release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) else {
                                    continue;
                                };
                                // Unfinished regiments or squadrons dragged
                                // out: their Move (A0 2026-11-15 "m-5": the
                                // regiment dropped on Fleet 3 boarded it).
                                let products: Vec<u64> = items
                                    .iter()
                                    .filter_map(|item| match *item {
                                        rebellion_render::defenses_window::DefensesItem::Pending {
                                            product: Some(product),
                                            ..
                                        } => Some(product),
                                        _ => None,
                                    })
                                    .collect();
                                if !products.is_empty() && products.len() == items.len() {
                                    let (destination, fleet) = match target {
                                        ReleaseTarget::System(system) => (system, None),
                                        ReleaseTarget::Fleet { fleet, system } => (system, Some(fleet)),
                                    };
                                    let blockaded =
                                        rebellion_core::products::move_asks(&world, &products);
                                    if confirmed || blockaded {
                                        move_confirmation_state.open(product_confirmation(
                                            &world,
                                            player_faction,
                                            products,
                                            (destination, fleet),
                                            blockaded,
                                        ));
                                    } else {
                                        panel_actions.push(PanelAction::MoveProducts {
                                            products,
                                            system: destination,
                                            fleet,
                                        });
                                    }
                                    continue;
                                }
                                let Some(team) = defenses_selection_team(&items) else {
                                    // hyp: an unfinished unit's move command
                                    // refuses (+0x50 bit 2 clear), and one
                                    // refusal refuses the order.
                                    msg_log.refuse(
                                        GameMessage::new(
                                            clock.tick,
                                            "Move refused".to_string(),
                                            MessageCategory::Event,
                                        ),
                                        UNTRACED_REFUSAL,
                                    );
                                    continue;
                                };
                                issue_team_move(
                                    &DefensesMoveContext {
                                        world: &world,
                                        movement: &movement_state,
                                        missions: &mission_state,
                                        transport: &troop_transport_state,
                                        deliveries: &delivery_state,
                                        blockaded: blockade_state.blockaded_systems(),
                                        faction: player_faction,
                                        tick: clock.tick,
                                    },
                                    &team,
                                    (system, SquadronPlace::Planet(system)),
                                    false,
                                    confirmed,
                                    target,
                                    &mut msg_log,
                                    &mut move_confirmation_state,
                                    &mut panel_actions,
                                );
                            }
                        }
                    }

                    for action in draw_missions_windows(
                        ctx,
                        &world,
                        fog_state,
                        &mission_state,
                        &mut missions_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        match action {
                            MissionsWindowAction::OpenSector(system) => {
                                sector_window_state.open_for_system(
                                    &world,
                                    system,
                                    cockpit_state.faction,
                                );
                                map_state.selected_system = Some(system);
                            }
                            MissionsWindowAction::SelectSystem(system) => {
                                map_state.selected_system = Some(system);
                            }
                            MissionsWindowAction::Minimize {
                                system,
                                logical_position,
                            } => system_window_state
                                .minimize_missions_window(system, logical_position),
                            // FUN_004ac5c0 with the window's slot 22
                            // (FUN_004a0400): the selected mission's menu,
                            // Encyclopedia, Status, Abort (A0
                            // missionstatus/02).
                            MissionsWindowAction::OpenObjectMenu { mission, point, .. } => {
                                let selection = Some(MenuObject::Mission(mission));
                                let mut gates = order_gates(
                                    selection,
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &delivery_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                    victory_state.conditions_word,
                                );
                                gates.abort = !abort_missions(
                                    &world,
                                    fog_state,
                                    &mission_state,
                                    selection,
                                    player_faction,
                                )
                                .is_empty();
                                object_menu = Some(ObjectMenuState::new(selection, gates, point));
                            }
                        }
                    }

                    // The menu commits on the release that starts targeting;
                    // only a later release may end the order.
                    let targeting_held = targeting.is_some();
                    // How a band's menu closes decides whether its band
                    // stays selected (A0 build-selection2/18, 21, 23).
                    let escaped = ctx.input(|input| input.key_pressed(egui_macroquad::egui::Key::Escape));
                    let band_menu = object_menu.as_ref().and_then(|menu| match menu.selection() {
                        Some(MenuObject::Producer { system, .. }) => Some(system),
                        _ => None,
                    });
                    // A Finder's menu takes the input over its modal Finder,
                    // and its Escape closes only the menu.
                    let finder_open = fleet_finder_state.is_open()
                        || troop_finder_state.is_open()
                        || personnel_finder_state.is_open()
                        || system_finder_state.is_open();
                    let menu_was_open = object_menu.is_some();
                    let menu_team: Vec<MenuObject> = object_menu
                        .as_ref()
                        .map(|menu| menu.team().to_vec())
                        .unwrap_or_default();
                    let band_menu_result = draw_object_menu(
                        ctx,
                        &mut object_menu,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        strategic_input_enabled || finder_open,
                    );
                    if menu_was_open && escaped {
                        ctx.input_mut(|input| {
                            input.consume_key(
                                egui_macroquad::egui::Modifiers::NONE,
                                egui_macroquad::egui::Key::Escape,
                            )
                        });
                    }
                    if let Some(system) = band_menu.filter(|_| object_menu.is_none()) {
                        let command = band_menu_result.as_ref().map(|(command, _)| *command);
                        if rebellion_render::system_window::band_menu_deselects(command, escaped) {
                            system_window_state.deselect_band(system);
                        }
                    }
                    // A menu for several objects gives its order the whole
                    // team (FUN_00487c50 builds it with the selection).
                    let band_menu_result = match band_menu_result {
                        Some((command, _)) if menu_team.len() > 1 => {
                            match team_confirmation(
                                &world,
                                &movement_state,
                                &mission_state,
                                player_faction,
                                command,
                                &menu_team,
                            ) {
                                Some(Ok(confirmation)) => {
                                    macroquad::logging::info!(
                                        "[interface] command={command:?} team={} status=asks",
                                        menu_team.len()
                                    );
                                    move_confirmation_state.open(confirmation);
                                }
                                Some(Err(refusal)) => {
                                    msg_log.push(GameMessage::new(
                                        clock.tick,
                                        refusal.to_string(),
                                        MessageCategory::Event,
                                    ));
                                }
                                None => {}
                            }
                            if let Some(order) = team_menu_order(
                                &world,
                                command,
                                &menu_team,
                                object_menu_place,
                            ) {
                                match order {
                                    TeamMenuOrder::Target(order) => {
                                        targeting = Some(Targeting::new(order));
                                    }
                                    TeamMenuOrder::Panel(action) => panel_actions.push(action),
                                }
                            }
                            None
                        }
                        result => result,
                    };
                    match band_menu_result {
                        // FUN_00486fb0 0x234: Planetary Assault of the
                        // selection's system by its fleets.
                        Some((ObjectMenuCommand::Assault, selection)) => {
                            if let Some((fleets, system)) = bombard_team(
                                &movement_state,
                                &world,
                                selection,
                                player_faction == MissionFaction::Alliance,
                            ) {
                                panel_actions.push(PanelAction::OrderAssault { fleets, system });
                            }
                        }
                        // FUN_00486fb0 0x260..0x263: the Command order on
                        // the selected character.
                        Some((
                            ObjectMenuCommand::GiveCommand(order),
                            Some(MenuObject::Character(character)),
                        )) => {
                            panel_actions.push(PanelAction::OrderCommand { character, order });
                        }
                        Some((ObjectMenuCommand::BombardmentTarget(kind), selection)) => {
                            if let Some((fleets, system)) = bombard_team(
                                &movement_state,
                                &world,
                                selection,
                                player_faction == MissionFaction::Alliance,
                            ) {
                                panel_actions.push(PanelAction::OrderBombardment {
                                    fleets,
                                    system,
                                    kind,
                                });
                            }
                        }
                        // FUN_0045d400 opens at the object's topic when the
                        // original window's source loaded.
                        Some((ObjectMenuCommand::Encyclopedia, selection)) => {
                            // A fleet opens its FLEETSD class "Fleet", listed
                            // in every index (FUN_00422620; A0
                            // menus2/20-fleet-encyclopedia.png).
                            match original_encyclopedia_catalog.as_ref() {
                                Some(catalog) => rebellion_render::open_original_encyclopedia_at(
                                    &mut enc_state,
                                    catalog,
                                    selection.and_then(|object| match object {
                                        // A mission opens its kind's MISSNSD
                                        // record.
                                        MenuObject::Mission(id) => mission_state
                                            .missions()
                                            .iter()
                                            .find(|mission| mission.id == id)
                                            .and_then(|mission| mission.kind.record_id())
                                            .map(rebellion_core::ids::DatId::raw),
                                        other => rebellion_render::original_encyclopedia_object(
                                            &world, other,
                                        ),
                                    }),
                                ),
                                None => enc_state.open = true,
                            }
                            macroquad::logging::info!(
                                "[interface] command=0x100 destination=encyclopedia status=opened topic_mode={} object={:?}",
                                enc_state.original_topic_mode,
                                enc_state.original_selected_object_id,
                            );
                        }
                        // FUN_00486fb0 0x103 → FUN_0042a440: the Status
                        // window (type 0x1a) for the selection.
                        Some((ObjectMenuCommand::Status, Some(selection)))
                            if selection.status_object().is_some() =>
                        {
                            let object = selection
                                .status_object()
                                .expect("the guard saw a Status object");
                            status_window_state.open(object);
                            let rect = rebellion_render::status_window::window_rect(cockpit_layout);
                            macroquad::logging::info!(
                                "[interface] command=0x103 destination=status_window status=opened object={:?} rect={},{},{},{}",
                                object,
                                rect.min.x,
                                rect.min.y,
                                rect.width(),
                                rect.height()
                            );
                        }
                        // FUN_00487740: the Retire order validates
                        // (FUN_004ed460 or FUN_00533ba0), then FUN_00487cc0
                        // asks (FUN_0049a350, case 0x242); a refusal goes
                        // to FUN_00487c90.
                        Some((ObjectMenuCommand::Retire, Some(object))) => {
                            if let Some(member) = object.mission_member() {
                                let allowed = retire_check(
                                    &world,
                                    &mission_state,
                                    member,
                                    player_faction == MissionFaction::Alliance,
                                );
                                macroquad::logging::info!(
                                    "[interface] command=0x242 member={member:?} result={:?}",
                                    allowed.as_ref().map(Result::is_ok),
                                );
                                match allowed {
                                    Some(Ok(name)) => {
                                        let (characters, special_forces) = match member {
                                            MissionMember::Character(key) => {
                                                (vec![key], Vec::new())
                                            }
                                            MissionMember::SpecialForce(key) => {
                                                (Vec::new(), vec![key])
                                            }
                                        };
                                        move_confirmation_state.open(MoveConfirmation {
                                            order: ConfirmedOrder::Retire,
                                            faction: player_faction,
                                            fleets: Vec::new(),
                                            characters,
                                            special_forces,
                                            missions: Vec::new(),
                                            scrap: Vec::new(),
                                            troops: Vec::new(),
                                            squadrons: Vec::new(),
        ships: None,
                                            blockaded: false,
                                            destination: SystemKey::default(),
                                            join: None,
                                            products: Vec::new(),
                                            lines: vec![(name, 0)],
                                        });
                                    }
                                    Some(Err(_)) => {
                                        msg_log.refuse(
                                            GameMessage::new(
                                                clock.tick,
                                                "Retire refused".to_string(),
                                                MessageCategory::Event,
                                            ),
                                            UNTRACED_REFUSAL,
                                        );
                                    }
                                    None => {}
                                }
                            }
                        }
                        // FUN_00487740: the Scrap validates (each member's
                        // +0x5c, scrap::check), then FUN_00487cc0 always
                        // asks for 0x200 (FUN_0049a350, case 0x200).
                        Some((ObjectMenuCommand::Scrap, Some(object))) => {
                            let side = Faction::of_alliance(
                                player_faction == MissionFaction::Alliance,
                            );
                            let team = object.scrap_team(&movement_state, &world, side);
                            if !team.is_empty() {
                                let allowed = team.iter().try_for_each(|&target| {
                                    rebellion_core::scrap::check_with_movement(
                                        &world,
                                        &movement_state,
                                        side,
                                        target,
                                    )
                                });
                                macroquad::logging::info!(
                                    "[interface] command=0x200 targets={team:?} result={allowed:?}"
                                );
                                if allowed.is_ok() {
                                    // FUN_0049a880 lists each member's name
                                    // (A0 menus3/12: three regiments).
                                    let lines = team
                                        .iter()
                                        .map(|&target| {
                                            (
                                                rebellion_core::scrap::target_name(&world, target)
                                                    .unwrap_or_default(),
                                                0,
                                            )
                                        })
                                        .collect();
                                    move_confirmation_state.open(MoveConfirmation {
                                        products: Vec::new(),
                                        order: ConfirmedOrder::Scrap,
                                        faction: player_faction,
                                        fleets: Vec::new(),
                                        characters: Vec::new(),
                                        special_forces: Vec::new(),
                                        missions: Vec::new(),
                                        scrap: team,
                                        troops: Vec::new(),
                                        squadrons: Vec::new(),
        ships: None,
                                        blockaded: false,
                                        destination: SystemKey::default(),
                                        join: None,
                                        lines,
                                    });
                                } else {
                                    msg_log.refuse(
                                        GameMessage::new(
                                            clock.tick,
                                            "Scrap refused".to_string(),
                                            MessageCategory::Event,
                                        ),
                                        UNTRACED_REFUSAL,
                                    );
                                }
                            }
                        }
                        // FUN_00487740: the Abort validates (FUN_0051ff30:
                        // every mission through FUN_00521600, refused with
                        // 0x40 in phase 4 and 0x41 in any phase but 8); a
                        // refusal asks nothing (FUN_00487c90, the advisor's
                        // reaction), else FUN_00487cc0 asks (FUN_0049a350,
                        // case 0x250).
                        Some((ObjectMenuCommand::Abort, selection @ Some(_))) => {
                            let missions = abort_missions(
                                &world,
                                fog_state,
                                &mission_state,
                                selection,
                                player_faction,
                            );
                            let allowed = !missions.is_empty()
                                && missions
                                    .iter()
                                    .all(|&id| mission_state.abort_allowed(id, player_faction));
                            if allowed {
                                macroquad::logging::info!(
                                    "[interface] command=0x250 missions={:?}",
                                    missions
                                );
                            } else {
                                macroquad::logging::info!(
                                    "[interface] command=0x250 missions={:?} status=refused reason=0x41",
                                    missions
                                );
                            }
                            if allowed {
                                move_confirmation_state.open(MoveConfirmation {
                                    order: ConfirmedOrder::Abort,
                                    faction: player_faction,
                                    fleets: Vec::new(),
                                    characters: Vec::new(),
                                    special_forces: Vec::new(),
                                    missions,
                                    scrap: Vec::new(),
                                    troops: Vec::new(),
                                    squadrons: Vec::new(),
        ships: None,
                                    blockaded: false,
                                    destination: SystemKey::default(),
                                    join: None,
                                    products: Vec::new(),
                                    lines: Vec::new(),
                                });
                            }
                        }
                        // FUN_00487c50 builds the order with the selection
                        // as its team; FUN_00429320 starts targeting.
                        Some((ObjectMenuCommand::Mission, Some(object))) => {
                            if let Some(member) = object.mission_member() {
                                targeting =
                                    Some(Targeting::new(TargetOrder::Mission(vec![member])));
                            }
                        }
                        // FUN_004ac730 → FUN_00486fb0: a move with no
                        // target goes to targeting too (FUN_00429320).
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Fleet(fleet)),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::FleetMove {
                                fleets: vec![fleet],
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                                icon: false,
                            }));
                        }
                        // FUN_00512700, kind 0x10: the order's team is the
                        // system's fleets of the player's side.
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::SystemIcon {
                                system,
                                quadrant: Quadrant::Fleets,
                            }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::FleetMove {
                                fleets: system_side_fleets(
                                    &movement_state,
                                    &world,
                                    system,
                                    player_faction == MissionFaction::Alliance,
                                ),
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                                icon: true,
                            }));
                        }
                        // FUN_00512700, kind 8: the order's team is the
                        // icon's entry, resolved when the target is set.
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::SystemIcon {
                                system,
                                quadrant: Quadrant::Defenses,
                            }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::DefensesMove {
                                system,
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        // FUN_00526c00: the headquarters' 0x201 and 0x202.
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Headquarters(_)),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::HeadquartersMove {
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        // An unfinished regiment's or squadron's Move
                        // (A0 2026-11-15 "u-3").
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(
                                MenuObject::PendingTroop {
                                    product: Some(product),
                                    ..
                                }
                                | MenuObject::PendingFighter {
                                    product: Some(product),
                                    ..
                                }
                                | MenuObject::PendingShip { product },
                            ),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::ProductMove {
                                products: vec![product],
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Troop(troop)),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::TroopMove {
                                troop,
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Character(character)),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::CharacterMove {
                                character,
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        // FUN_00536bc0: a unit's Move is a character's.
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::SpecialForce(unit)),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::SpecialForceMove {
                                unit,
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Fighter { fleet, index }),
                        )) => {
                            if let Some(entry) = world
                                .fleets
                                .get(fleet)
                                .and_then(|value| value.fighters.get(index))
                            {
                                targeting = Some(Targeting::new(TargetOrder::SquadronMove {
                                    class: entry.class,
                                    from: SquadronPlace::Fleet(fleet),
                                    confirmed: command == ObjectMenuCommand::ConfirmedMove,
                                }));
                            }
                        }
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::PlanetFighter { system, class }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::SquadronMove {
                                class,
                                from: SquadronPlace::Planet(system),
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        Some((
                            command @ (ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove),
                            Some(MenuObject::Ship {
                                fleet,
                                index,
                                roster,
                            }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::ShipMove {
                                fleet,
                                ships: vec![index],
                                roster,
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        // FUN_00580b00: Create Fleet runs at once, against the
                        // ships' own system.
                        Some((
                            ObjectMenuCommand::CreateFleet,
                            Some(MenuObject::Ship {
                                fleet,
                                index,
                                roster,
                            }),
                        )) => panel_actions.push(PanelAction::CreateFleet {
                            fleet,
                            ships: vec![index],
                            roster,
                        }),
                        Some((
                            ObjectMenuCommand::Destination,
                            Some(MenuObject::SystemIcon {
                                system,
                                quadrant: Quadrant::System,
                            }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::Destination {
                                system,
                                area: None,
                            }));
                        }
                        // A band's manager takes the order for its own area
                        // (manual p. 83, "select Destination from this menu
                        // and then click the targeting cross hairs").
                        Some((
                            ObjectMenuCommand::Destination,
                            Some(MenuObject::Producer { system, area }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::Destination {
                                system,
                                area: Some(area),
                            }));
                        }
                        // Build (0x210..0x212): FUN_0041d640 opens Build
                        // Selection on what the band's manager lists.
                        Some((
                            ObjectMenuCommand::Build,
                            Some(MenuObject::Producer { system, area }),
                        )) => {
                            let is_alliance = player_faction == MissionFaction::Alliance;
                            let kinds = rebellion_render::build_selection::in_list_order(
                                &world,
                                rebellion_core::build_selection::listed_classes(
                                &world,
                                &research_state,
                                area,
                                is_alliance,
                            ),
                            );
                            if build_selection_state.open(system, area, is_alliance, kinds) {
                                macroquad::logging::info!(
                                    "[interface] command=0x21x destination=build_selection status=opened area={area:?} system={}",
                                    world.systems.get(system).map_or("", |value| value.name.as_str())
                                );
                            }
                        }
                        // Stop (0x213): FUN_00487cc0 asks first
                        // (FUN_0049a350 case 0x213, A0 build-selection2/09):
                        // the band's units and their name.
                        Some((
                            ObjectMenuCommand::Stop,
                            Some(MenuObject::Producer { system, area }),
                        )) => {
                            let building = mfg_state
                                .queue(system, area)
                                .and_then(|queue| Some((queue.active()?.kind, queue.len())));
                            if let Some((kind, count)) = building {
                                let name =
                                    rebellion_render::manufacturing_window::product(&world, kind).0;
                                move_confirmation_state.open(MoveConfirmation {
                                    order: ConfirmedOrder::Stop { area },
                                    faction: player_faction,
                                    fleets: Vec::new(),
                                    characters: Vec::new(),
                                    special_forces: Vec::new(),
                                    missions: Vec::new(),
                                    scrap: Vec::new(),
                                    troops: Vec::new(),
                                    squadrons: Vec::new(),
                                    ships: None,
                                    blockaded: false,
                                    destination: system,
                                    join: None,
                                    products: Vec::new(),
                                    lines: vec![(name, u32::try_from(count).unwrap_or(u32::MAX))],
                                });
                            }
                        }
                        // ManuMgr_vt1fc (0x216) sets the manager's Reserved
                        // to the order's value: the row's mark flips (A0
                        // menus3/32, 33).
                        Some((
                            ObjectMenuCommand::Reserved,
                            Some(MenuObject::Producer { system, area }),
                        )) => panel_actions.push(PanelAction::ToggleReserved {
                            system,
                            area: Some(area),
                        }),
                        // A facility icon's Reserved: every manager of the
                        // system takes the opposite of the row's mark
                        // (Class_006612a8_vt50, Event216_Fire).
                        Some((
                            ObjectMenuCommand::Reserved,
                            Some(MenuObject::SystemIcon {
                                system,
                                quadrant: Quadrant::System,
                            }),
                        )) => {
                            panel_actions.push(PanelAction::ToggleReserved { system, area: None })
                        }
                        // FUN_00486fb0 0x203 → FUN_0041d600 → FUN_00429350:
                        // the edit opens in the window that lists the object.
                        // port: with no Fleet window open there, one opens.
                        Some((
                            ObjectMenuCommand::Rename,
                            Some(
                                object @ (MenuObject::Fleet(_)
                                | MenuObject::Ship { .. }
                                | MenuObject::PendingShip { .. }),
                            ),
                        )) => {
                            let entry = match object {
                                // An unfinished ship is renamed in its
                                // fleet's tree, which opens for it.
                                MenuObject::PendingShip { product } => world
                                    .products
                                    .get(product)
                                    .and_then(|value| match value.container {
                                        rebellion_core::products::Container::Fleet(fleet) => {
                                            Some(rebellion_render::fleet_window::FleetWindowEntry::Product {
                                                fleet,
                                                product,
                                            })
                                        }
                                        rebellion_core::products::Container::System(_) => None,
                                    }),
                                MenuObject::Ship { fleet, index, .. } => {
                                    Some(rebellion_render::fleet_window::FleetWindowEntry::Ship {
                                        fleet,
                                        index,
                                    })
                                }
                                MenuObject::Fleet(fleet) => Some(
                                    rebellion_render::fleet_window::FleetWindowEntry::Fleet(fleet),
                                ),
                                _ => None,
                            };
                            if let Some((entry, system)) = entry.and_then(|entry| {
                                rebellion_core::movement::listed_location(
                                    &movement_state,
                                    &world,
                                    entry.fleet(),
                                )
                                .map(|system| (entry, system))
                            }) {
                                if fleet_window_state.selection(system).is_none() {
                                    sector_window_state.open_for_system(
                                        &world,
                                        system,
                                        cockpit_state.faction,
                                    );
                                    if let Some(point) = sector_window_state.fleet_window_point(
                                        &world,
                                        cockpit_layout,
                                        system,
                                    ) {
                                        fleet_window_state.open(
                                            &world,
                                            system,
                                            point,
                                            cockpit_state.faction,
                                            cockpit_layout,
                                        );
                                    }
                                }
                                if matches!(
                                    entry,
                                    rebellion_render::fleet_window::FleetWindowEntry::Product { .. }
                                ) {
                                    fleet_window_state.expand(system, entry.fleet());
                                }
                                fleet_window_state.begin_rename(&world, system, entry);
                            }
                        }
                        // port: the other items are drawn disabled.
                        Some(_) | None => {}
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let (Some(request), Some(menu), Some(rect)) = (
                        interface_fixture_request,
                        object_menu.as_ref(),
                        rebellion_render::object_menu::object_menu_rect(ctx),
                    ) {
                        interface_test_fixture::emit_object_menu(
                            request,
                            (
                                rect,
                                rebellion_render::object_menu::object_submenu_rect(ctx),
                            ),
                            menu,
                            &world,
                            &sector_window_state,
                            (
                                &defenses_window_state,
                                &system_window_state,
                                &fleet_window_state,
                            ),
                        );
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_mission_flow(
                            request,
                            &world,
                            &mission_dialog_state,
                            &mission_state,
                        );
                    }

                    match draw_mission_dialog(
                        ctx,
                        &world,
                        &mut mission_dialog_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        Some(MissionDialogAction::Begin {
                            kind,
                            faction,
                            team,
                            decoys,
                            target,
                        }) => panel_actions.push(PanelAction::DispatchMission {
                            kind,
                            faction,
                            team,
                            decoys,
                            target: target.system,
                            target_character: target.character,
                            target_object: target.object,
                            tick: clock.tick,
                        }),
                        Some(MissionDialogAction::Encyclopedia) => enc_state.open = true,
                        None => {}
                    }

                    match draw_build_selection(
                        ctx,
                        &world,
                        &mfg_state,
                        delivery_state.en_route(),
                        &mut build_selection_state,
                        cockpit_layout,
                        &mut bmp_cache,
                        rebellion_render::resource_monitors::maintenance_unallocated(
                            &world,
                            &mfg_state,
                            &delivery_state,
                            &blockade_state,
                            &uprising_state,
                            cockpit_state.faction,
                        ),
                        &|fleet| troop_transport_state.carried_count(fleet),
                    ) {
                        Some(BuildSelectionAction::Confirm {
                            system,
                            area,
                            kind,
                            count,
                            agent,
                        }) => {
                            // FUN_00487740 → FUN_0043a2a0: the agent's own
                            // order earns its acknowledgement (bit 1).
                            if agent {
                                if let Some(schedule) = advisor_schedule.as_mut() {
                                    schedule.add_flags(
                                        rebellion_render::advisor_schedule::reaction::BUILD_CONFIRMED,
                                    );
                                }
                                macroquad::logging::info!(
                                    "[agent] build status=confirmed count={}",
                                    count
                                );
                            }
                            panel_actions.push(PanelAction::BuildProduction {
                                system,
                                area,
                                kind,
                                count,
                            });
                        }
                        Some(BuildSelectionAction::Encyclopedia) => enc_state.open = true,
                        None => {}
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_production(
                            request,
                            &world,
                            &mfg_state,
                            delivery_state.en_route(),
                            &system_window_state,
                            &sector_window_state,
                            &build_selection_state,
                            &|fleet| troop_transport_state.carried_count(fleet),
                        );
                    }

                    // FUN_00429440: the Finder's choice opens its system's
                    // sector window, then the Fleet window from the fleet
                    // icon with the fleet or ship selected.
                    if let Some(FleetFinderAction::Open(entry)) = draw_fleet_finder(
                        ctx,
                        &world,
                        &movement_state,
                        fog_state,
                        &mut fleet_finder_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        // A fleet in hyperspace is its destination's child
                        // (FUN_00556390), so its windows open there.
                        // A copied fleet opens at the system its copy is of.
                        let copied = match entry {
                            rebellion_render::fleet_window::FleetWindowEntry::Copy { system, .. } => Some(system),
                            _ => None,
                        };
                        if let Some(system) = copied.or_else(|| {
                            rebellion_core::movement::listed_location(
                                &movement_state,
                                &world,
                                entry.fleet(),
                            )
                        }) {
                            sector_window_state.open_for_system(
                                &world,
                                system,
                                cockpit_state.faction,
                            );
                            if let Some(point) = sector_window_state.fleet_window_point(
                                &world,
                                cockpit_layout,
                                system,
                            ) {
                                map_state.selected_system = Some(system);
                                fleet_window_state.open(
                                    &world,
                                    system,
                                    point,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                                fleet_window_state.select(system, entry);
                            }
                        }
                    }

                    // FUN_0046d8d0 / FUN_00465d80: a Finder's choice goes
                    // through FUN_00429440, which opens the target's sector
                    // window and then the window for its kind.
                    let troop_choice = draw_troop_finder(
                        ctx,
                        &world,
                        fog_state,
                        &mut troop_finder_state,
                        cockpit_layout,
                        &mut bmp_cache,
                        &troop_transport_state,
                    )
                    .map(|action| match action {
                        TroopFinderAction::OpenDefenses { system } => {
                            (system, FinderTarget::Regiments)
                        }
                        TroopFinderAction::OpenFleet { system, fleet } => {
                            (system, FinderTarget::Fleet(fleet))
                        }
                    });
                    // Display, Enter or a double click: FUN_00429ce0 opens the
                    // window of the chosen system's sector; nothing is
                    // selected.
                    if let Some(rebellion_render::system_finder::SystemFinderAction::OpenSector(
                        system,
                    )) = rebellion_render::system_finder::draw_system_finder(
                        ctx,
                        &world,
                        &mut system_finder_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        sector_window_state.open_for_system(&world, system, cockpit_state.faction);
                        macroquad::logging::info!(
                            "[interface] system_finder status=display system={}",
                            world.systems.get(system).map_or("", |value| value.name.as_str())
                        );
                    }
                    personnel_finder_state.set_destinations(
                        mission_state
                            .en_route()
                            .iter()
                            .filter_map(|transit| match transit.member {
                                MissionMember::Character(key) => Some((key, transit.to)),
                                MissionMember::SpecialForce(_) => None,
                            })
                            .collect(),
                    );
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_personnel_finder(
                            request,
                            &world,
                            fog_state,
                            &personnel_finder_state,
                        );
                    }
                    let personnel_choice = draw_personnel_finder(
                        ctx,
                        &world,
                        fog_state,
                        &mut personnel_finder_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    )
                    .and_then(|action| match action {
                        // FUN_00429440 family 0x30 (character_target).
                        PersonnelFinderAction::OpenCharacter { character, system } => {
                            use rebellion_render::personnel_finder::{
                                character_target, CharacterTarget,
                            };
                            match world.characters.get(character).map(character_target) {
                                Some(CharacterTarget::Fleet(fleet)) => {
                                    rebellion_core::movement::listed_location(
                                        &movement_state,
                                        &world,
                                        fleet,
                                    )
                                    .map(|system| (system, FinderTarget::Fleet(fleet)))
                                }
                                Some(CharacterTarget::Missions) => system.map(|system| {
                                    (
                                        system,
                                        FinderTarget::Missions(
                                            rebellion_core::missions::MissionMember::Character(
                                                character,
                                            ),
                                        ),
                                    )
                                }),
                                Some(CharacterTarget::Defenses) | None => {
                                    system.map(|system| (system, FinderTarget::Defenses))
                                }
                            }
                        }
                        // FUN_00429440 for a special force at its system:
                        // its running mission's window, else the Defenses
                        // window. port: special forces have no hidden
                        // mission flag.
                        PersonnelFinderAction::OpenSystem { system, unit } => {
                            match unit.and_then(|key| world.special_forces.get(key).map(|sf| (key, sf))) {
                                Some((key, sf)) if sf.on_mission => Some((
                                    system,
                                    FinderTarget::Missions(
                                        rebellion_core::missions::MissionMember::SpecialForce(key),
                                    ),
                                )),
                                Some(_) => Some((system, FinderTarget::Defenses)),
                                None => Some((system, FinderTarget::Sector)),
                            }
                        }
                        PersonnelFinderAction::OpenFleet { system, fleet } => {
                            Some((system, FinderTarget::Fleet(fleet)))
                        }
                        // FUN_00429440 on the player's copy of the
                        // character: its copied fleet, its copied system,
                        // or nothing.
                        PersonnelFinderAction::OpenCopy(place) => {
                            use rebellion_render::fleet_window::FleetWindowEntry;
                            use rebellion_render::personnel_finder::CopyPlace;
                            match place {
                                CopyPlace::Fleet(entry @ FleetWindowEntry::Copy { system, .. }) => {
                                    Some((system, FinderTarget::CopiedFleet(entry)))
                                }
                                CopyPlace::Fleet(_) | CopyPlace::Nowhere => None,
                                CopyPlace::System(system) => {
                                    Some((system, FinderTarget::Defenses))
                                }
                                CopyPlace::Missions(system) => {
                                    Some((system, FinderTarget::MissionsWindow))
                                }
                            }
                        }
                    });
                    // A right-button release on a Finder opens the object
                    // menu with nothing selected (FUN_004ac5c0, FUN_00461740).
                    if object_menu.is_none() {
                        object_menu = ObjectMenuState::for_finder(ctx, cockpit_layout, |point| {
                            fleet_finder_state.contains_screen_point(cockpit_layout, point)
                                || troop_finder_state.contains_screen_point(cockpit_layout, point)
                                || system_finder_state.contains_screen_point(cockpit_layout, point)
                                || personnel_finder_state
                                    .contains_screen_point(cockpit_layout, point)
                        });
                    }
                    if let Some((system, target)) = troop_choice.or(personnel_choice) {
                        sector_window_state.open_for_system(&world, system, cockpit_state.faction);
                        map_state.selected_system = Some(system);
                        match target {
                            FinderTarget::Sector => {}
                            FinderTarget::Defenses | FinderTarget::Regiments => {
                                if let Some(point) = sector_window_state.quadrant_window_point(
                                    &world,
                                    cockpit_layout,
                                    system,
                                    rebellion_render::quadrant_icons::Quadrant::Defenses,
                                ) {
                                    defenses_window_state.open(
                                        &world,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                    if matches!(target, FinderTarget::Regiments) {
                                        defenses_window_state.show_page(
                                            system,
                                            rebellion_render::defenses_window::DefensesPage::Regiments,
                                        );
                                    }
                                }
                            }
                            // FUN_00429440 kind 11, then slot 27
                            // (FUN_004a1e10) shows the member.
                            FinderTarget::Missions(member) => {
                                if let Some(point) = sector_window_state.quadrant_window_point(
                                    &world,
                                    cockpit_layout,
                                    system,
                                    rebellion_render::quadrant_icons::Quadrant::Missions,
                                ) {
                                    missions_window_state.open(
                                        &world,
                                        fog_state,
                                        &mission_state,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                    missions_window_state.show_member(
                                        &mission_state,
                                        system,
                                        member,
                                    );
                                }
                            }
                            FinderTarget::MissionsWindow => {
                                if let Some(point) = sector_window_state.quadrant_window_point(
                                    &world,
                                    cockpit_layout,
                                    system,
                                    rebellion_render::quadrant_icons::Quadrant::Missions,
                                ) {
                                    missions_window_state.open(
                                        &world,
                                        fog_state,
                                        &mission_state,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                }
                            }
                            FinderTarget::Fleet(fleet) => {
                                if let Some(point) = sector_window_state.fleet_window_point(
                                    &world,
                                    cockpit_layout,
                                    system,
                                ) {
                                    fleet_window_state.open(
                                        &world,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                    fleet_window_state.select(
                                        system,
                                        rebellion_render::fleet_window::FleetWindowEntry::Fleet(
                                            fleet,
                                        ),
                                    );
                                }
                            }
                            FinderTarget::CopiedFleet(entry) => {
                                if let Some(point) = sector_window_state.fleet_window_point(
                                    &world,
                                    cockpit_layout,
                                    system,
                                ) {
                                    fleet_window_state.open(
                                        &world,
                                        system,
                                        point,
                                        cockpit_state.faction,
                                        cockpit_layout,
                                    );
                                    fleet_window_state.select(system, entry);
                                }
                            }
                        }
                    }

                    let status_sources = StatusSources {
                        world: &world,
                        missions: &mission_state,
                        movement: &movement_state,
                        transport: &troop_transport_state,
                        manufacturing: &mfg_state,
                        today: clock.tick,
                        objectives: rebellion_render::status_rows::ObjectiveSource {
                            alliance: cockpit_state.faction
                                == rebellion_render::cockpit::CockpitFaction::Alliance,
                            conditions_word: victory_state.conditions_word,
                            standard: campaign_config.victory_conditions
                                == VictoryConditions::Standard,
                        },
                    };
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_status_rows(
                            request,
                            status_sources,
                            &status_window_state,
                        );
                    }
                    // FUN_004443a0: 0x66 closes the Status window and opens
                    // the Encyclopedia; 0x65 closes it.
                    if let Some(action) = draw_status_window(
                        ctx,
                        status_sources,
                        &mut status_window_state,
                        cockpit_state.faction,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        macroquad::logging::info!(
                            "[interface] destination=status_window status=closed action={}",
                            match action {
                                StatusWindowAction::Encyclopedia => "encyclopedia",
                                StatusWindowAction::Close => "close",
                            }
                        );
                        if action == StatusWindowAction::Encyclopedia {
                            enc_state.open = true;
                        }
                    }

                    // FUN_0044f5e0: the checkmark resubmits with force 1,
                    // which validates again and departs without asking.
                    let answer = draw_move_confirmation(
                        ctx,
                        &mut move_confirmation_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    );
                    // FUN_0041ce20(order, 1): each mission validates again
                    // and event 0x261 ends it (FUN_005233a0 -> +0x1dc(2)).
                    // FUN_0041ce20(order, 1): the Stop drops what the band
                    // builds (manual p. 84).
                    if let Some(MoveConfirmationAction::Stop { system, area }) = answer {
                        panel_actions.push(PanelAction::StopProduction { system, area });
                    }
                    if let Some(MoveConfirmationAction::Abort { missions }) = &answer {
                        for &id in missions {
                            let aborted = mission_state.abort_allowed(id, player_faction)
                                && mission_state.abort(id);
                            macroquad::logging::info!(
                                "[interface] command=0x250 destination=abort mission={} result={}",
                                id,
                                aborted
                            );
                        }
                    }
                    // FUN_0041ce20(order, 1): the Scrap validates every
                    // member again (FUN_0051ff30), then destroys each with
                    // reason 0x14 and refunds half its refined cost
                    // (FUN_00534c00, FUN_00530270); a refusal goes to
                    // FUN_00487c90.
                    if let Some(MoveConfirmationAction::Scrap { targets }) = &answer {
                        let side = Faction::of_alliance(player_faction == MissionFaction::Alliance);
                        // FUN_0041ce20(order, 1) validates the order again:
                        // a fleet gone into hyperspace meanwhile refuses.
                        let result = targets
                            .iter()
                            .try_for_each(|&target| {
                                rebellion_core::scrap::check_with_movement(
                                    &world,
                                    &movement_state,
                                    side,
                                    target,
                                )
                            })
                            .and_then(|()| {
                                rebellion_core::scrap::scrap_team(
                                    &mut world,
                                    &mut production_state,
                                    &mut troop_transport_state,
                                    side,
                                    targets,
                                )
                            });
                        // A scrapped ship's regiments beyond the room left
                        // are lost.
                        troop_transport_state.destroy_untransportable_cargo(&mut world);
                        macroquad::logging::info!(
                            "[interface] command=0x200 destination=scrap targets={:?} result={:?}",
                            targets,
                            result
                        );
                        if result.is_err() {
                            msg_log.refuse(
                                GameMessage::new(
                                    clock.tick,
                                    "Scrap refused".to_string(),
                                    MessageCategory::Event,
                                ),
                                UNTRACED_REFUSAL,
                            );
                        }
                    }
                    if let Some(MoveConfirmationAction::MoveHeadquarters { destination }) = &answer {
                        panel_actions.push(PanelAction::MoveHeadquarters {
                            destination: *destination,
                        });
                    }
                    if let Some(MoveConfirmationAction::Retire {
                        characters,
                        special_forces,
                    }) = &answer
                    {
                        panel_actions.extend(
                            characters
                                .iter()
                                .map(|&key| MissionMember::Character(key))
                                .chain(
                                    special_forces
                                        .iter()
                                        .map(|&key| MissionMember::SpecialForce(key)),
                                )
                                .map(PanelAction::Retire),
                        );
                    }
                    if let Some(MoveConfirmationAction::Confirm {
                        fleets,
                        characters,
                        special_forces,
                        troops,
                        squadrons,
                        ships,
                        products,
                        destination,
                        join,
                    }) = answer
                    {
                        // An unfinished unit's 0x202 is its Move again
                        // (FUN_0041ce20(order, 1); products::move_to_target).
                        if !products.is_empty() {
                            panel_actions.push(PanelAction::MoveProducts {
                                products: products.clone(),
                                system: destination,
                                fleet: join,
                            });
                        }
                        // FUN_0041ce20(order, 1): a ship's 0x202 is its Move
                        // again: onto the fleet, or out to the system.
                        if let Some(ShipTeam {
                            fleet,
                            ships,
                            roster,
                        }) = ships
                        {
                            panel_actions.push(match join {
                                Some(target) => PanelAction::JoinFleet {
                                    mover: FleetMover::Ships {
                                        fleet,
                                        ships,
                                        roster,
                                    },
                                    target,
                                },
                                None => PanelAction::MoveShips {
                                    fleet,
                                    ships,
                                    roster,
                                    system: destination,
                                },
                            });
                        }
                        // FUN_0041ce20(order, 1): a squadron's 0x202 is its
                        // Move again, to the fleet or the planet.
                        panel_actions.extend(squadrons.into_iter().map(|(class, from)| {
                            PanelAction::MoveSquadron {
                                class,
                                from,
                                to: join.map_or(
                                    SquadronPlace::Planet(destination),
                                    SquadronPlace::Fleet,
                                ),
                            }
                        }));
                        // FUN_0041ce20(order, 1): a regiment's 0x202 is its
                        // Move again, to the fleet or the system.
                        panel_actions.extend(troops.into_iter().map(|troop| {
                            PanelAction::MoveRegiment {
                                troop,
                                target: join.map_or(
                                    RegimentTarget::System(destination),
                                    RegimentTarget::Fleet,
                                ),
                            }
                        }));
                        if characters.len() + special_forces.len() > 1 {
                            // A defenses icon's 0x202: its one 0x241 takes
                            // every character and special force
                            // (FUN_0053c810), one Move mission.
                            panel_actions.push(PanelAction::MoveMembers {
                                members: characters
                                    .into_iter()
                                    .map(MissionMember::Character)
                                    .chain(
                                        special_forces.into_iter().map(MissionMember::SpecialForce),
                                    )
                                    .collect(),
                                system: destination,
                                fleet: join,
                            });
                        } else {
                            panel_actions.extend(special_forces.into_iter().map(|unit| {
                                PanelAction::MoveSpecialForce {
                                    unit,
                                    system: destination,
                                    fleet: join,
                                }
                            }));
                            // FUN_0041ce20(order, 1): a character's 0x202 is
                            // the Move mission order 0x241 again.
                            panel_actions.extend(characters.into_iter().map(|character| {
                                PanelAction::MoveCharacter {
                                    character,
                                    system: destination,
                                    fleet: join,
                                }
                            }));
                        }
                        panel_actions.extend(fleets.into_iter().map(|fleet| match join {
                            Some(target) => PanelAction::JoinFleet {
                                mover: FleetMover::Fleet(fleet),
                                target,
                            },
                            None => PanelAction::DispatchFleet {
                                fleet,
                                destination,
                                troops: Vec::new(),
                            },
                        }));
                    }

                    if targeting.is_some() {
                        capture_pointer(ctx);
                        let (mouse_x, mouse_y) = mouse_position();
                        let pointer = ctx
                            .input(|input| input.pointer.latest_pos())
                            .unwrap_or(egui_macroquad::egui::pos2(mouse_x, mouse_y));
                        targeting_cursor_drawn =
                            draw_targeting_cursor(ctx, &mut bmp_cache, cockpit_layout, pointer);
                        // FUN_00422ce0's WM_LBUTTONUP in mode 2 targets what
                        // the window under the point answers, then
                        // FUN_0042a320 opens the dialog with the kinds the
                        // team may undertake; with none, nothing opens.
                        if targeting_held && is_mouse_button_released(MouseButton::Left) {
                            let windows = ReleaseWindows {
                                sector: &sector_window_state,
                                system: &system_window_state,
                                fleet: &fleet_window_state,
                                defenses: &defenses_window_state,
                                missions: &missions_window_state,
                            };
                            // A Mission asks the window's +0x68, the object
                            // under the point; every other order its +0x70.
                            if let Some(TargetOrder::Mission(team)) =
                                targeting.as_ref().map(Targeting::order)
                            {
                                let team = team.clone();
                                targeting = None;
                                if let Some(target) = mission_release_target(
                                    ctx,
                                    MissionReleaseSources {
                                        world: &world,
                                        movement: &movement_state,
                                        fog: fog_state,
                                        missions: &mission_state,
                                        manufacturing: &mfg_state,
                                        deliveries: &delivery_state,
                                        transport: &troop_transport_state,
                                    },
                                    cockpit_layout,
                                    windows,
                                    &team,
                                    pointer,
                                ) {
                                    let kinds = rebellion_core::missions::available_kinds(
                                        &world,
                                        &uprising_state,
                                        &research_state,
                                        player_faction,
                                        &team,
                                        &[],
                                        target,
                                    );
                                    mission_dialog_state.open(
                                        player_faction,
                                        target,
                                        team,
                                        rebellion_render::mission_dialog::box_order(kinds),
                                    );
                                }
                            }
                            let destination = release_destination(
                                ctx,
                                &world,
                                &movement_state,
                                fog_state,
                                cockpit_layout,
                                windows,
                                pointer,
                            );
                            match targeting.take().map(|order| order.release(destination)) {
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::Mission(_),
                                    ..
                                }) => {}
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::FleetMove {
                                            fleets,
                                            confirmed,
                                            icon,
                                        },
                                    target,
                                }) => issue_fleet_move(
                                    &FleetMoveContext {
                                        world: &world,
                                        movement: &movement_state,
                                        blockaded: blockade_state.blockaded_systems(),
                                        faction: player_faction,
                                        tick: clock.tick,
                                        icon,
                                    },
                                    &fleets,
                                    confirmed,
                                    target,
                                    &mut msg_log,
                                    &mut move_confirmation_state,
                                    &mut panel_actions,
                                ),
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::ShipMove {
                                            fleet,
                                            ships,
                                            roster,
                                            confirmed: true,
                                        },
                                    target,
                                }) => confirm_ship_move(
                                    &world,
                                    player_faction,
                                    ShipTeam {
                                        fleet,
                                        ships,
                                        roster,
                                    },
                                    target,
                                    &mut move_confirmation_state,
                                ),
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::ShipMove {
                                            fleet,
                                            ships,
                                            roster,
                                            confirmed: false,
                                        },
                                    target: ReleaseTarget::Fleet { fleet: target, .. },
                                }) => panel_actions.push(PanelAction::JoinFleet {
                                    mover: FleetMover::Ships {
                                        fleet,
                                        ships,
                                        roster,
                                    },
                                    target,
                                }),
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::ShipMove {
                                            fleet,
                                            ships,
                                            roster,
                                            confirmed: false,
                                        },
                                    target: ReleaseTarget::System(system),
                                }) => panel_actions.push(PanelAction::MoveShips {
                                    fleet,
                                    ships,
                                    roster,
                                    system,
                                }),
                                // A character's 0x201 -> 0x241 against the
                                // release window's +0x70: a system, or a
                                // fleet to board.
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::CharacterMove {
                                            character,
                                            confirmed,
                                        },
                                    target,
                                }) => {
                                    let (system, fleet) = match target {
                                        ReleaseTarget::System(system) => (system, None),
                                        ReleaseTarget::Fleet { fleet, system } => {
                                            (system, Some(fleet))
                                        }
                                    };
                                    let asks = confirmed
                                        || world.characters.get(character).is_some_and(|c| {
                                            c.current_fleet.is_none()
                                                && move_out_of_blockade_asks(
                                                    &world,
                                                    blockade_state.blockaded_systems(),
                                                    c.current_system,
                                                    player_faction,
                                                )
                                        });
                                    if asks {
                                        confirm_character_move(
                                            &world,
                                            player_faction,
                                            character,
                                            system,
                                            fleet,
                                            clock.tick,
                                            &mut msg_log,
                                            &mut move_confirmation_state,
                                        );
                                    } else {
                                        panel_actions.push(PanelAction::MoveCharacter {
                                            character,
                                            system,
                                            fleet,
                                        });
                                    }
                                }
                                // A special force's 0x201 -> 0x241, as a
                                // character's: onto a fleet, it boards.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::SpecialForceMove { unit, confirmed },
                                    target,
                                }) => {
                                    let (system, fleet) = match target {
                                        ReleaseTarget::System(system) => (system, None),
                                        ReleaseTarget::Fleet { fleet, system } => {
                                            (system, Some(fleet))
                                        }
                                    };
                                    let asks = confirmed
                                        || move_out_of_blockade_asks(
                                            &world,
                                            blockade_state.blockaded_systems(),
                                            world
                                                .systems
                                                .iter()
                                                .find(|(_, sys)| sys.special_forces.contains(&unit))
                                                .map(|(key, _)| key),
                                            player_faction,
                                        );
                                    if asks {
                                        confirm_special_force_move(
                                            &world,
                                            player_faction,
                                            unit,
                                            system,
                                            fleet,
                                            clock.tick,
                                            &mut msg_log,
                                            &mut move_confirmation_state,
                                        );
                                    } else {
                                        panel_actions.push(PanelAction::MoveSpecialForce {
                                            unit,
                                            system,
                                            fleet,
                                        });
                                    }
                                }
                                // A defenses icon's 0x201 or 0x202 against
                                // the release window's +0x70.
                                // A window selection's 0x201 against the
                                // release window's +0x70.
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::TeamMove {
                                            system,
                                            from,
                                            team,
                                            confirmed,
                                        },
                                    target,
                                }) => issue_team_move(
                                    &DefensesMoveContext {
                                        world: &world,
                                        movement: &movement_state,
                                        missions: &mission_state,
                                        transport: &troop_transport_state,
                                        deliveries: &delivery_state,
                                        blockaded: blockade_state.blockaded_systems(),
                                        faction: player_faction,
                                        tick: clock.tick,
                                    },
                                    &team,
                                    (system, from),
                                    false,
                                    confirmed,
                                    target,
                                    &mut msg_log,
                                    &mut move_confirmation_state,
                                    &mut panel_actions,
                                ),
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::DefensesMove { system, confirmed },
                                    target,
                                }) => issue_defenses_move(
                                    &DefensesMoveContext {
                                        world: &world,
                                        movement: &movement_state,
                                        missions: &mission_state,
                                        transport: &troop_transport_state,
                                        deliveries: &delivery_state,
                                        blockaded: blockade_state.blockaded_systems(),
                                        faction: player_faction,
                                        tick: clock.tick,
                                    },
                                    system,
                                    confirmed,
                                    target,
                                    &mut msg_log,
                                    &mut move_confirmation_state,
                                    &mut panel_actions,
                                ),
                                // The headquarters' 0x201 against the
                                // release window's +0x70. port: a Fleet
                                // window's fleet is refused, the HQ only
                                // standing in systems.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::HeadquartersMove { confirmed },
                                    target,
                                }) => match target {
                                    ReleaseTarget::System(destination) => {
                                        order_headquarters_move(
                                            &world,
                                            &troop_transport_state,
                                            player_faction,
                                            destination,
                                            confirmed,
                                            clock.tick,
                                            &mut msg_log,
                                            &mut move_confirmation_state,
                                            &mut panel_actions,
                                        );
                                    }
                                    ReleaseTarget::Fleet { .. } => {
                                        macroquad::logging::info!(
                                            "[interface] command=0x201 headquarters status=refused reason=fleet"
                                        );
                                    }
                                },
                                // An unfinished unit's 0x201: FUN_00556390
                                // moves an object that is not mobile at
                                // once (products::move_to_target). port:
                                // Confirmed Move asks nothing (no trip, so
                                // no days to list).
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::ProductMove { products, confirmed },
                                    target,
                                }) => {
                                    let (system, fleet) = match target {
                                        ReleaseTarget::System(system) => (system, None),
                                        ReleaseTarget::Fleet { fleet, system } => {
                                            (system, Some(fleet))
                                        }
                                    };
                                    // FUN_00487cc0: 0x202 always asks; 0x201
                                    // asks when the first member stands in a
                                    // blockaded system of its side.
                                    let blockaded = rebellion_core::products::move_asks(&world, &products);
                                    if confirmed || blockaded {
                                        move_confirmation_state.open(product_confirmation(
                                            &world,
                                            player_faction,
                                            products,
                                            (system, fleet),
                                            blockaded,
                                        ));
                                    } else {
                                        panel_actions.push(PanelAction::MoveProducts {
                                            products,
                                            system,
                                            fleet,
                                        });
                                    }
                                }
                                // A regiment's 0x201 against the release
                                // window's +0x70 (FUN_004a3130 for a Fleet
                                // window), boarding, landing or travelling.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::TroopMove { troop, confirmed },
                                    target,
                                }) => {
                                    let (system, regiment_target) = match target {
                                        ReleaseTarget::Fleet { fleet, system } => {
                                            (system, RegimentTarget::Fleet(fleet))
                                        }
                                        ReleaseTarget::System(system) => {
                                            (system, RegimentTarget::System(system))
                                        }
                                    };
                                    let asks = confirmed
                                        || move_out_of_blockade_asks(
                                            &world,
                                            blockade_state.blockaded_systems(),
                                            world
                                                .systems
                                                .iter()
                                                .find(|(_, sys)| sys.ground_units.contains(&troop))
                                                .map(|(key, _)| key),
                                            player_faction,
                                        );
                                    if asks {
                                        confirm_troop_move(
                                            &world,
                                            &movement_state,
                                            &troop_transport_state,
                                            blockade_state.blockaded_systems(),
                                            player_faction,
                                            troop,
                                            (system, regiment_target),
                                            clock.tick,
                                            &mut msg_log,
                                            &mut move_confirmation_state,
                                        );
                                    } else {
                                        panel_actions.push(PanelAction::MoveRegiment {
                                            troop,
                                            target: regiment_target,
                                        });
                                    }
                                }
                                // A squadron's 0x201 against the release
                                // window's +0x70: boarding or landing.
                                Some(TargetingEnd::Target {
                                    order:
                                        TargetOrder::SquadronMove {
                                            class,
                                            from,
                                            confirmed,
                                        },
                                    target,
                                }) => {
                                    let on_planet = match from {
                                        SquadronPlace::Planet(system) => Some(system),
                                        SquadronPlace::Fleet(_) => None,
                                    };
                                    let asks = confirmed
                                        || move_out_of_blockade_asks(
                                            &world,
                                            blockade_state.blockaded_systems(),
                                            on_planet,
                                            player_faction,
                                        );
                                    if asks {
                                        confirm_squadron_move(
                                            &world,
                                            &troop_transport_state.squadrons,
                                            player_faction,
                                            (class, from),
                                            target,
                                            clock.tick,
                                            &mut msg_log,
                                            &mut move_confirmation_state,
                                        );
                                    } else {
                                        panel_actions.push(PanelAction::MoveSquadron {
                                            class,
                                            from,
                                            to: target.squadron_place(),
                                        });
                                    }
                                }
                                // FUN_00512700 kind 4: the order's team is
                                // every production area of the system, or
                                // a band's own area.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::Destination { system, area },
                                    target: ReleaseTarget::System(destination),
                                }) => panel_actions.push(PanelAction::SetDestination {
                                    system,
                                    area,
                                    destination,
                                }),
                                // FUN_00537180: a construction manager's
                                // (0xa0..0xa1) Destination must be a system,
                                // else 1/0x25; a shipyard or training
                                // manager takes a fleet of its side.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::Destination { system, area },
                                    target: ReleaseTarget::Fleet { fleet, system: at },
                                }) => match rebellion_core::manufacturing::fleet_destination_area(
                                    &world,
                                    area,
                                    fleet,
                                    player_faction == MissionFaction::Alliance,
                                ) {
                                    Ok(area) => panel_actions.push(PanelAction::SetFleetDestination {
                                        system,
                                        area,
                                        fleet,
                                        at,
                                        // A ship's entry names the ship (0x14..0x1b).
                                        ship: rebellion_render::targeting::release_ship(
                                            ctx,
                                            &world,
                                            &movement_state,
                                            fog_state,
                                            cockpit_layout,
                                            windows,
                                            pointer,
                                        )
                                        .filter(|&(of, _)| of == fleet)
                                        .map(|(_, ship)| ship),
                                    }),
                                    Err(refusal) => macroquad::logging::info!(
                                        "[interface] command=0x214 destination=production_destination status=refused reason={}",
                                        refusal.status().map_or_else(|| "other_side".to_owned(), |status| format!("{status:#x}"))
                                    ),
                                },
                                // FUN_0049e380 takes the destination;
                                // the agent then finds the producer,
                                // points it there and opens Build
                                // Selection (FUN_0049e6b0).
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::AgentBuild(kind),
                                    target,
                                }) => {
                                    let fleet = match target {
                                        ReleaseTarget::Fleet { fleet, .. } => Some(fleet),
                                        ReleaseTarget::System(_) => None,
                                    };
                                    let target = rebellion_core::agent_build::BuildTarget {
                                        system: target.system(),
                                        fleet,
                                    };
                                    let side = rebellion_core::dat::Faction::from(player_faction);
                                    let outcome = match rebellion_core::agent_build::accept_target(
                                        &world, side, kind, target,
                                    ) {
                                        Ok(request) => rebellion_core::agent_build::run(
                                            &world,
                                            &mfg_state,
                                            &uprising_state,
                                            side,
                                            request,
                                        ),
                                        Err(refusal) => {
                                            rebellion_core::agent_build::BuildOutcome::Refused(refusal)
                                        }
                                    };
                                    match outcome {
                                        rebellion_core::agent_build::BuildOutcome::Open {
                                            producer,
                                            area,
                                            destination,
                                        } => {
                                            panel_actions.push(PanelAction::SetDestination {
                                                system: producer,
                                                area: Some(area),
                                                destination,
                                            });
                                            let is_alliance = player_faction == MissionFaction::Alliance;
                                            let kinds = rebellion_render::build_selection::in_list_order(
                                &world,
                                rebellion_core::build_selection::listed_classes(
                                                &world,
                                                &research_state,
                                                area,
                                                is_alliance,
                                            ),
                                            );
                                            let opened = build_selection_state
                                                .open_for_agent(producer, area, is_alliance, kinds);
                                            macroquad::logging::info!(
                                                "[agent] build kind={:?} status={} producer={} destination={}",
                                                kind,
                                                if opened { "opened" } else { "no_classes" },
                                                world.systems.get(producer).map_or("", |v| v.name.as_str()),
                                                world.systems.get(destination).map_or("", |v| v.name.as_str())
                                            );
                                        }
                                        rebellion_core::agent_build::BuildOutcome::Refused(refusal) => {
                                            if let Some(schedule) = advisor_schedule.as_mut() {
                                                schedule.refused(
                                                    refusal.status(),
                                                    false,
                                                    advisor_now(&clock),
                                                );
                                            }
                                            macroquad::logging::info!(
                                                "[agent] build kind={:?} status=refused reason={:?}",
                                                kind,
                                                refusal
                                            );
                                        }
                                    }
                                }
                                Some(TargetingEnd::Dropped) | None => {}
                            }
                        }
                    }

                    // The replacement message and status bars covered the
                    // original command controls. Keep those reconstructed
                    // surfaces out of parity mode until their bitmap-driven
                    // versions are restored.

                    // Droid advisor (floating window, bottom-right)
                    draw_advisor(ctx, &mut advisor_state);

                    // The galaxy view holds the capture while targeting.
                    let cockpit_command = (!original_modal_fixture_open
                        && !modal_window_open
                        && targeting.is_none())
                        .then(|| {
                            handle_cockpit_egui_input(ctx, &mut cockpit_state, &mut bmp_cache)
                        });
                    if let Some(btn) = cockpit_command.flatten() {
                        let (command, destination) = match btn {
                            CockpitButton::SystemFinder => {
                                // FUN_0042a000 opens one System Finder; F2
                                // reaches the same case (FUN_00422ce0).
                                system_finder_state.open(cockpit_state.faction);
                                macroquad::logging::info!(
                                    "[interface] command=0x12d destination=system_finder status=opened_original"
                                );
                                return;
                            }
                            CockpitButton::FleetFinder => {
                                // FUN_0042a0c0 opens one Fleet Finder; F3
                                // reaches the same case (FUN_00422ce0).
                                fleet_finder_state.open(cockpit_state.faction);
                                macroquad::logging::info!(
                                    "[interface] command=0x12e destination=fleet_finder status=opened_original"
                                );
                                return;
                            }
                            CockpitButton::PersonnelFinder => {
                                // FUN_0042a0c0's 0x12f case; F5 reaches it.
                                personnel_finder_state.open(cockpit_state.faction);
                                macroquad::logging::info!(
                                    "[interface] command=0x12f destination=personnel_finder status=opened_original"
                                );
                                return;
                            }
                            CockpitButton::TroopFinder => {
                                // FUN_0042a0c0's 0x130 case; F4 reaches it.
                                troop_finder_state.open(cockpit_state.faction);
                                macroquad::logging::info!(
                                    "[interface] command=0x130 destination=troop_finder status=opened_original"
                                );
                                return;
                            }
                            CockpitButton::GameOptions => (0x133, "game_options"),
                            // FUN_00429f30 (command 0x131, F7) opens the
                            // original window when its source loaded.
                            CockpitButton::Encyclopedia
                                if original_encyclopedia_catalog.is_some()
                                    && original_encyclopedia_topics[usize::from(
                                        cockpit_state.faction == CockpitFaction::Empire,
                                    )]
                                    .is_some() =>
                            {
                                // A new window each time (FUN_0045d400):
                                // All Databases, nothing selected
                                // (FUN_0045f100(0x6f, 1)).
                                if let Some(catalog) = original_encyclopedia_catalog.as_ref() {
                                    rebellion_render::open_original_encyclopedia_at(
                                        &mut enc_state,
                                        catalog,
                                        None,
                                    );
                                }
                                macroquad::logging::info!(
                                    "[interface] command=0x131 destination=encyclopedia status=opened_original"
                                );
                                return;
                            }
                            CockpitButton::Encyclopedia => (0x131, "encyclopedia"),
                            CockpitButton::GalacticInformationDisplay => {
                                cockpit_state.gid_ui.menu_open = !cockpit_state.gid_ui.menu_open;
                                cockpit_state.gid_ui.category = None;
                                macroquad::logging::info!(
                                    "[interface] command=0x{:x} destination=gid_menu status={}",
                                    rebellion_render::strategic_gid_control(cockpit_state.faction)
                                        .command_id,
                                    if cockpit_state.gid_ui.menu_open {
                                        "opened_original"
                                    } else {
                                        "closed"
                                    }
                                );
                                return;
                            }
                        };
                        if btn == CockpitButton::GameOptions {
                            save_slots = read_save_slots(&saves_dir);
                            game_options_state.set_origin(GameOptionsOrigin::CommandCenter);
                            game_options_state.player_is_alliance =
                                player_faction == MissionFaction::Alliance;
                            game_options_state.refresh_saves(&save_slots);
                            game_mode = GameMode::GameOptions {
                                origin: GameOptionsOrigin::CommandCenter,
                            };
                            macroquad::logging::info!(
                                "[interface] command=0x{:x} destination={} status=opened_original",
                                command,
                                destination
                            );
                        } else {
                            macroquad::logging::info!(
                                "[interface] command=0x{:x} destination={} status=pending_original_window",
                                command,
                                destination
                            );
                        }
                    }
                });
                egui_macroquad::draw();

                // The Battle Summary, a child window of the Command Center
                // (FUN_0042a640 -> FUN_0046f140), over the live strategy
                // screen: the galaxy, the cockpit and its open windows.
                if tactical_state.battle_summary_open() {
                    let action = rebellion_render::draw_battle_summary_window(
                        &mut tactical_state,
                        &mut bmp_cache,
                        &world,
                        &troop_transport_state,
                    );
                    if let Some(lines) = tactical_state.take_summary_log(&world) {
                        macroquad::logging::info!("[tactical-results] summary lines={:?}", lines);
                    }
                    if !matches!(action, TacticalAction::None) {
                        battle_summary_action = Some(action);
                    }
                }
            }

            GameMode::TacticalCombat => {
                let tac_action = draw_tactical_view(
                    &mut tactical_state,
                    &mut bmp_cache,
                    &world,
                    &troop_transport_state,
                );
                // The Battle Summary's lines, once, as its page draws them.
                if let Some(lines) = tactical_state.take_summary_log(&world) {
                    macroquad::logging::info!("[tactical-results] summary lines={:?}", lines);
                }
                // Battle Alert's Retreat (0x67) answers "simulate" with the
                // player's retreat order (FUN_00450540, FUN_00487830).
                let player_retreats = matches!(tac_action, TacticalAction::RetreatBeforeBattle);
                match tac_action {
                    TacticalAction::BeginCombat => {
                        // Advance from placement to combat phase.
                        if let Some(ref mut session) = tactical_state.session {
                            session.phase = rebellion_render::BattlePhase::Combat;
                            session.queue_battle_ready_voice();
                        }
                    }
                    TacticalAction::AutoResolve | TacticalAction::RetreatBeforeBattle => {
                        // Simulate Results (0x65) or Retreat (0x67): the
                        // abstract battle (FUN_004362b0 answer 1, state 8),
                        // then the original Battle Results window.
                        let mut result_presented = false;
                        if let Some(session) = tactical_state.session.clone() {
                            let battle_before =
                                rebellion_core::loyalty::battle_watch(&world, session.system);
                            let attacker_is_alliance = world
                                .fleets
                                .get(session.attacker_fleet)
                                .map(|f| f.is_alliance);
                            let combat_rolls: Vec<f64> =
                                (0..256).map(|_| sim_rng.gen::<f64>()).collect();
                            // FUN_004362b0: the AI side's retreat order
                            // (FUN_00485d50) and the player's button.
                            let ais: Vec<&AIState> =
                                std::iter::once(&ai_state).chain(&secondary_ai_state).collect();
                            let mut retreat_allowed = rebellion_core::ai_strategy::retreat_orders(
                                &ais,
                                &world,
                                &movement_state,
                                &troop_transport_state,
                                session.system,
                            );
                            let player_is_alliance =
                                session.player_is_attacker == session.attacker_is_alliance;
                            retreat_allowed[usize::from(!player_is_alliance)] = player_retreats;
                            // Every fleet of both sides fights (FUN_0040c780).
                            let (space_result, _) =
                                rebellion_core::abstract_battle::resolve_space_battle(
                                    &world,
                                    session.system,
                                    &rebellion_core::abstract_battle::side_fleets(
                                        &world,
                                        session.system,
                                        session.attacker_fleet,
                                    ),
                                    &rebellion_core::abstract_battle::side_fleets(
                                        &world,
                                        session.system,
                                        session.defender_fleet,
                                    ),
                                    rebellion_core::abstract_battle::abstract_seed(&combat_rolls),
                                    session.start_tick,
                                    retreat_allowed,
                                );
                            apply_space_combat_result(&space_result, &mut world, &combat_rolls);
                            rebellion_core::squadrons::recover(
                                &mut world,
                                &std::mem::take(&mut battle_sorties),
                            );
                            let winner = battle_winner(space_result.winner, attacker_is_alliance);
                            rebellion_core::loyalty::settle_battle(
                                &mut world,
                                battle_before,
                                session.system,
                                winner,
                            );
                            // FUN_00516ef0: the loser's fleets and planet
                            // squadrons leave on Autorouting.
                            let withdrawals = rebellion_core::abstract_battle::withdraw_loser(
                                &mut world,
                                &mut movement_state,
                                &mut troop_transport_state.squadrons,
                                session.system,
                                winner,
                                clock.tick,
                            );
                            // FUN_0050a0b0 -> FUN_00510b80: the side that withdrew changes,
                            // and the garrison takes the system's withdraw percent.
                            BlockadeSystem::after_withdrawal(
                                &mut blockade_state,
                                &world,
                                session.system,
                                &withdrawals,
                            );
                            troop_transport_state.destroy_untransportable_cargo(&mut world);

                            let winner_str = match space_result.winner {
                                CombatSide::Attacker if session.attacker_is_alliance => {
                                    "Alliance victory"
                                }
                                CombatSide::Attacker => "Empire victory",
                                CombatSide::Defender if session.attacker_is_alliance => {
                                    "Empire victory"
                                }
                                CombatSide::Defender => "Alliance victory",
                                CombatSide::Draw => "Draw",
                            };
                            msg_log.push(GameMessage::at_system(
                                session.start_tick,
                                format!(
                                    "Space battle at {} — {} (auto-resolved)",
                                    session.system_name, winner_str
                                ),
                                MessageCategory::Combat,
                                session.system,
                            ));

                            // Advisor: combat result
                            let player_won = match space_result.winner {
                                CombatSide::Attacker => session.player_is_attacker,
                                CombatSide::Defender => !session.player_is_attacker,
                                CombatSide::Draw => false,
                            };
                            advisor_combat_result(
                                &mut advisor_state,
                                &session.system_name,
                                player_won,
                            );
                            result_presented =
                                tactical_state.present_auto_resolve_result(&space_result);
                        }
                        if !result_presented {
                            game_mode = GameMode::Galaxy;
                        }
                    }
                    action @ (TacticalAction::ReturnToGalaxy
                    | TacticalAction::OpenBattleSystem(_)
                    | TacticalAction::OpenBattleFleet(_)) => {
                        battle_summary_action = Some(action);
                    }
                    TacticalAction::TogglePause => {
                        if let Some(ref mut session) = tactical_state.session {
                            session.paused = !session.paused;
                        }
                    }
                    TacticalAction::SetSpeed(speed) => {
                        if let Some(ref mut session) = tactical_state.session {
                            session.combat_speed = speed.clamp(1, 4);
                        }
                    }
                    // FUN_005f1170 -> command 0x7a -> FUN_005ad530.
                    TacticalAction::WithdrawFromBattle => {
                        if let Some(ref mut session) = tactical_state.session {
                            session.begin_player_withdrawal();
                            rebellion_render::tactical_view::withdraw_player_fighters(session);
                            session.selected_ship = None;
                            session.selected_fighter_group = None;
                        }
                    }
                    TacticalAction::OpenGameOptions => {
                        save_slots = read_save_slots(&saves_dir);
                        game_options_state.set_origin(GameOptionsOrigin::TacticalBattle);
                        game_options_state.refresh_saves(&save_slots);
                        game_mode = GameMode::GameOptions {
                            origin: GameOptionsOrigin::TacticalBattle,
                        };
                        macroquad::logging::info!(
                            "[interface] command=0x133 destination=game_options status=opened_original"
                        );
                    }
                    TacticalAction::None => {}
                }

                if game_mode == GameMode::TacticalCombat {
                    if let Some(outcome) = tactical_state.take_pending_trench_run_cinematic() {
                        cutscene_player = open_cutscene(
                            Path::new(trench_run_cutscene_path(outcome)),
                            audio_vol.cutscene_volume(),
                            #[cfg(not(target_arch = "wasm32"))]
                            &mut audio_engine,
                            #[cfg(target_arch = "wasm32")]
                            browser_menu_audio.as_mut(),
                        );
                        if cutscene_player.is_some() {
                            game_mode = GameMode::Cutscene {
                                kind: CutsceneKind::TrenchRun(outcome),
                            };
                        }
                    }
                }
            }

        }

        // The Battle Summary's Close (0xc9), system (0xcb) and fleet (0xca)
        // buttons (FUN_00470ea0): the battle's result goes to the galaxy.
        if let Some(tac_action) = battle_summary_action.take() {
            let requested_result_system = match &tac_action {
                TacticalAction::OpenBattleSystem(system) => Some(*system),
                _ => None,
            };
            let requested_result_fleet = match &tac_action {
                TacticalAction::OpenBattleFleet(fleet) => Some(*fleet),
                _ => None,
            };
            let strategic_results_applied = tactical_state.strategic_results_applied();
            // Apply combat results from tactical session to GameWorld.
            if let Some(session) = tactical_state.end_battle() {
                // A blockade the battle leaves is silent (bit
                // 0x4000, blockade_started_message).
                battled_systems.insert(session.system);
                let battle_return = if strategic_results_applied {
                    tactical_flow::summarize_results(&session)
                } else {
                    let before = tactical_flow::persistence_snapshot(&session, &world);
                    let result = tactical_flow::apply_results(
                        &session,
                        &mut world,
                        &mut troop_transport_state,
                    );
                    let after = tactical_flow::persistence_snapshot(&session, &world);
                    macroquad::logging::info!(
                        "[tactical_results] strategic_persistence applied=true attacker_present={}->{} defender_present={}->{} attacker_capitals={}->{} defender_capitals={}->{} attacker_fighters={}->{} defender_fighters={}->{} attacker_death_star={}->{} defender_death_star={}->{}",
                        before.attacker_present,
                        after.attacker_present,
                        before.defender_present,
                        after.defender_present,
                        before.attacker_capital_ships,
                        after.attacker_capital_ships,
                        before.defender_capital_ships,
                        after.defender_capital_ships,
                        before.attacker_fighter_squadrons,
                        after.attacker_fighter_squadrons,
                        before.defender_fighter_squadrons,
                        after.defender_fighter_squadrons,
                        before.attacker_has_death_star,
                        after.attacker_has_death_star,
                        before.defender_has_death_star,
                        after.defender_has_death_star,
                    );
                    result
                };
                rebellion_core::squadrons::recover(
                    &mut world,
                    &std::mem::take(&mut battle_sorties),
                );
                if !strategic_results_applied {
                    // FUN_00516ef0 (state 9 follows the tactical
                    // battle too): the loser's fleets leave.
                    let winner = match battle_return.winner {
                        Some(rebellion_render::CombatWinner::Attacker) => {
                            Some(Faction::of_alliance(session.attacker_is_alliance))
                        }
                        Some(rebellion_render::CombatWinner::Defender) => {
                            Some(Faction::of_alliance(!session.attacker_is_alliance))
                        }
                        Some(rebellion_render::CombatWinner::Draw) | None => None,
                    };
                    let withdrawals = rebellion_core::abstract_battle::withdraw_loser(
                        &mut world,
                        &mut movement_state,
                        &mut troop_transport_state.squadrons,
                        session.system,
                        winner,
                        clock.tick,
                    );
                    // FUN_0050a0b0 -> FUN_00510b80: the side that withdrew changes,
                    // and the garrison takes the system's withdraw percent.
                    BlockadeSystem::after_withdrawal(
                        &mut blockade_state,
                        &world,
                        session.system,
                        &withdrawals,
                    );
                    let winner_str = match battle_return.winner {
                        Some(rebellion_render::CombatWinner::Attacker) => {
                            "Attacker victory"
                        }
                        Some(rebellion_render::CombatWinner::Defender) => {
                            "Defender victory"
                        }
                        Some(rebellion_render::CombatWinner::Draw) | None => "Draw",
                    };
                    msg_log.push(GameMessage::at_system(
                        session.start_tick,
                        format!(
                            "Space battle at {} — {} (tactical)",
                            session.system_name, winner_str
                        ),
                        MessageCategory::Combat,
                        session.system,
                    ));

                    advisor_combat_result(
                        &mut advisor_state,
                        &session.system_name,
                        battle_return.player_won,
                    );
                }

                // Auto-resolve already completed bombardment and
                // ground resolution before presenting its result.
                game_mode = GameMode::Galaxy;
            } else {
                game_mode = GameMode::Galaxy;
            }

            if game_mode == GameMode::Galaxy {
                if let Some(system) = requested_result_system {
                    map_state.selected_system = Some(system);
                    if open_battle_result_sector(
                        &world,
                        system,
                        &mut sector_window_state,
                        cockpit_state.faction,
                    ) {
                        macroquad::logging::info!(
                            "[tactical-results] sector_window system={} windows={}",
                            world.systems.get(system).map_or("", |s| s.name.as_str()),
                            sector_window_state.window_count(),
                        );
                    }
                }
                // 0xca: the system's sector window, then the
                // fleet's Fleet window with the fleet selected,
                // as the Fleet Finder opens it (FUN_00429440).
                if let Some(fleet) = requested_result_fleet {
                    if let Some(value) = world.fleets.get(fleet) {
                        let system = value.location;
                        map_state.selected_system = Some(system);
                        let _ = open_battle_result_sector(
                            &world,
                            system,
                            &mut sector_window_state,
                            cockpit_state.faction,
                        );
                        let layout = cockpit_state.layout();
                        if fleet_window_state.open(
                            &world,
                            system,
                            (85, 55),
                            cockpit_state.faction,
                            layout,
                        ) {
                            fleet_window_state.select(
                                system,
                                rebellion_render::fleet_window::FleetWindowEntry::Fleet(
                                    fleet,
                                ),
                            );
                            macroquad::logging::info!(
                                "[tactical-results] fleet_window system={} ships={} selected=fleet",
                                world.systems.get(system).map_or("", |s| s.name.as_str()),
                                value.capital_ships.iter().filter(|ship| ship.alive).count(),
                            );
                        }
                    }
                }
            }
        }

        // The session log starts once a new or loaded game runs in the
        // galaxy between days (rebellion_data::session).
        if session_needs_start && game_mode == GameMode::Galaxy && ai_day.loyalty_before.is_none() {
            let start = LiveCampaign {
                world: &mut world,
                clock: &mut clock,
                manufacturing: &mut mfg_state,
                missions: &mut mission_state,
                events: &mut event_state,
                ai: &mut ai_state,
                movement: &mut movement_state,
                fog_alliance: &mut fog_alliance_state,
                fog_empire: &mut fog_empire_state,
                player_faction: &mut player_faction,
                blockade: &mut blockade_state,
                uprising: &mut uprising_state,
                research: &mut research_state,
                victory: &mut victory_state,
                economy: &mut economy_state,
                sim_rng: &mut sim_rng,
                ai2: &mut secondary_ai_state,
                repair: &mut repair_state,
                production: &mut production_state,
                autoscrap: &mut autoscrap_state,
                informant: &mut informant_state,
                healing: &mut healing_state,
                force_discovery: &mut force_discovery_state,
                dagobah: &mut dagobah_state,
                final_battle: &mut final_battle_state,
                jabba: &mut jabba_state,
                seat_of_power: &mut seat_of_power_state,
                troop_transport: &mut troop_transport_state,
                deliveries: &mut delivery_state,
                player_agent: &mut player_agent,
                combat_cooldowns: &mut combat_cooldowns,
                campaign_config: &mut campaign_config,
                ai_day: &mut ai_day,
                day_rolls: &mut day_rolls,
            }
            .snapshot();
            session_log.restart(start);
            session_needs_start = false;
            session_speed = Some(clock.speed);
        }
        let session_moment = order_moment(&ai_day);

        // 5. Apply panel actions. A held pause keeps the player's orders
        // until the game runs (FUN_0041e300 stops the frame step before
        // FUN_0051df30 executes them).
        let panel_actions = rebellion_render::panels::admit_orders(
            &mut held_orders,
            clock.holding(),
            panel_actions,
        );
        for action in panel_actions {
            match action {
                PanelAction::CheckSessionReplay => {
                    let live = LiveCampaign {
                        world: &mut world,
                        clock: &mut clock,
                        manufacturing: &mut mfg_state,
                        missions: &mut mission_state,
                        events: &mut event_state,
                        ai: &mut ai_state,
                        movement: &mut movement_state,
                        fog_alliance: &mut fog_alliance_state,
                        fog_empire: &mut fog_empire_state,
                        player_faction: &mut player_faction,
                        blockade: &mut blockade_state,
                        uprising: &mut uprising_state,
                        research: &mut research_state,
                        victory: &mut victory_state,
                        economy: &mut economy_state,
                        sim_rng: &mut sim_rng,
                        ai2: &mut secondary_ai_state,
                        repair: &mut repair_state,
                        production: &mut production_state,
                        autoscrap: &mut autoscrap_state,
                        informant: &mut informant_state,
                        healing: &mut healing_state,
                        force_discovery: &mut force_discovery_state,
                        dagobah: &mut dagobah_state,
                        final_battle: &mut final_battle_state,
                        jabba: &mut jabba_state,
                        seat_of_power: &mut seat_of_power_state,
                        troop_transport: &mut troop_transport_state,
                        deliveries: &mut delivery_state,
                        player_agent: &mut player_agent,
                        combat_cooldowns: &mut combat_cooldowns,
                        campaign_config: &mut campaign_config,
                        ai_day: &mut ai_day,
                        day_rolls: &mut day_rolls,
                    }
                    .snapshot();
                    let text = session_check(&session_log, &live, order_moment(&ai_day));
                    macroquad::logging::info!("[session] {text}");
                    msg_log.push(GameMessage::new(clock.tick, text, MessageCategory::Event));
                }
                PanelAction::SaveGame { slot, name } => {
                    let state = LiveCampaign {
                        world: &mut world,
                        clock: &mut clock,
                        manufacturing: &mut mfg_state,
                        missions: &mut mission_state,
                        events: &mut event_state,
                        ai: &mut ai_state,
                        movement: &mut movement_state,
                        fog_alliance: &mut fog_alliance_state,
                        fog_empire: &mut fog_empire_state,
                        player_faction: &mut player_faction,
                        blockade: &mut blockade_state,
                        uprising: &mut uprising_state,
                        research: &mut research_state,
                        victory: &mut victory_state,
                        economy: &mut economy_state,
                        sim_rng: &mut sim_rng,
                        ai2: &mut secondary_ai_state,
                        repair: &mut repair_state,
                        production: &mut production_state,
                        autoscrap: &mut autoscrap_state,
                        informant: &mut informant_state,
                        healing: &mut healing_state,
                        force_discovery: &mut force_discovery_state,
                        dagobah: &mut dagobah_state,
                        final_battle: &mut final_battle_state,
                        jabba: &mut jabba_state,
                        seat_of_power: &mut seat_of_power_state,
                        troop_transport: &mut troop_transport_state,
                        deliveries: &mut delivery_state,
                        player_agent: &mut player_agent,
                        combat_cooldowns: &mut combat_cooldowns,
                        campaign_config: &mut campaign_config,
                        ai_day: &mut ai_day,
                        day_rolls: &mut day_rolls,
                    }
                    .snapshot();
                    let active_mods = mod_runtime.enabled_mod_list();
                    match rebellion_data::save::save_slot(
                        &saves_dir,
                        slot,
                        &name,
                        &state,
                        &active_mods,
                    ) {
                        Ok(fingerprint) => {
                            macroquad::logging::info!(
                                "save_state_fingerprint slot={} tick={} fingerprint={}",
                                slot,
                                state.clock.tick,
                                fingerprint
                            );
                            game_options_state.save_in_flight = Some(slot);
                            if let Some((slot, true)) = settle_game_options_save(
                                &mut game_options_state,
                                &mut save_slots,
                                &saves_dir,
                                rebellion_data::save::save_write_status(),
                            ) {
                                msg_log.push(GameMessage::new(
                                    clock.tick,
                                    format!("Saved game to slot {}", slot + 1),
                                    MessageCategory::Event,
                                ));
                            }
                        }
                        Err(error) => {
                            macroquad::logging::warn!("[save] slot={} failed: {}", slot, error);
                            // FUN_00408240: "Selected Game Failed To Save".
                            game_options_state.saved_notice = Some(false);
                        }
                    }
                }
                PanelAction::LoadGame { slot } => {
                    match rebellion_data::save::load_slot(&saves_dir, slot) {
                        Ok((meta, state)) => {
                            held_orders.clear();
                            session_log.clear();
                            session_needs_start = true;
                            drop_left_ending(&mut pending_ending);
                            advisor_schedule = None;
                            advisor_state.clips.clear();
                            briefing_caption = None;
                            macroquad::logging::info!(
                                "load_state_fingerprint slot={} tick={} fingerprint={}",
                                slot,
                                meta.game_tick,
                                meta.state_fingerprint
                            );
                            let mut campaign = LiveCampaign {
                                world: &mut world,
                                clock: &mut clock,
                                manufacturing: &mut mfg_state,
                                missions: &mut mission_state,
                                events: &mut event_state,
                                ai: &mut ai_state,
                                movement: &mut movement_state,
                                fog_alliance: &mut fog_alliance_state,
                                fog_empire: &mut fog_empire_state,
                                player_faction: &mut player_faction,
                                blockade: &mut blockade_state,
                                uprising: &mut uprising_state,
                                research: &mut research_state,
                                victory: &mut victory_state,
                                economy: &mut economy_state,
                                sim_rng: &mut sim_rng,
                                ai2: &mut secondary_ai_state,
                                repair: &mut repair_state,
                                production: &mut production_state,
                                autoscrap: &mut autoscrap_state,
                                informant: &mut informant_state,
                                healing: &mut healing_state,
                                force_discovery: &mut force_discovery_state,
                                dagobah: &mut dagobah_state,
                                final_battle: &mut final_battle_state,
                                jabba: &mut jabba_state,
                                seat_of_power: &mut seat_of_power_state,
                                troop_transport: &mut troop_transport_state,
                                deliveries: &mut delivery_state,
                                player_agent: &mut player_agent,
                                combat_cooldowns: &mut combat_cooldowns,
                                campaign_config: &mut campaign_config,
                                ai_day: &mut ai_day,
                                day_rolls: &mut day_rolls,
                            };
                            campaign.restore(state);
                            let restored_fingerprint =
                                rebellion_data::save::compute_state_fingerprint(
                                    &campaign.snapshot(),
                                );
                            game_speed_ui = GameSpeedUiState::default();
                            macroquad::logging::info!(
                                "[campaign] loaded configuration={}",
                                campaign_config.summary()
                            );
                            warmed_galaxy_font_sizes.clear();

                            cockpit_state.faction = if player_faction == MissionFaction::Alliance {
                                CockpitFaction::Alliance
                            } else {
                                CockpitFaction::Empire
                            };
                            advisor_state.set_faction(AdvisorFaction::from(cockpit_state.faction));
                            // FUN_00439550: the advisor's schedule and update
                            // flag come back with the agent.
                            advisor_schedule = loaded_advisor_schedule(
                                &gdata_path,
                                AdvisorFaction::from(cockpit_state.faction),
                                &player_agent,
                            );
                            map_state = GalaxyMapState::default();
                            sector_window_state.clear();
                            system_window_state.clear();
                            fleet_window_state.clear();
                            defenses_window_state.clear();
                            assault_summary_state.clear();
                            missions_window_state.clear();
                            object_menu = None;
                            targeting = None;
                            build_selection_state = build_selection_state.for_next_game();
                            mission_dialog_state = MissionDialogState::default();
                            move_confirmation_state = MoveConfirmationState::default();
                            status_window_state = StatusWindowState::default();
                            fleet_finder_state = FleetFinderState::default();
                            troop_finder_state = TroopFinderState::default();
                            personnel_finder_state = PersonnelFinderState::default();
                            system_finder_state = rebellion_render::system_finder::SystemFinderState::default();
                            message_index = None;
                            advice_saved_speed = None;
                            enc_state = EncyclopediaState::new();
                            enc_state.set_edata_path(configured_edata_path(&gdata_path));
                            enc_state.set_asset_profile(asset_render_profile);
                            enc_state.set_hd_path(
                                gdata_path
                                    .parent()
                                    .unwrap_or(Path::new("."))
                                    .join("hd")
                                    .join("EData"),
                            );
                            tactical_state = TacticalState::new();
                            dual_ai_mode = secondary_ai_state.is_some();
                            msg_log = MessageLog::default();
                            msg_log.push(GameMessage::new(
                                clock.tick,
                                format!("Loaded save ‘{}’ from slot {}", meta.name, slot + 1),
                                MessageCategory::Event,
                            ));
                            game_mode = GameMode::Galaxy;
                            // The loaded game's galaxy window starts its
                            // score (`FUN_00422ce0`). port: the counter and
                            // mood are not saved, so it starts as a new
                            // game's.
                            galaxy_score = rebellion_render::GalaxyScore::default();
                            galaxy_mood_day = None;
                            #[cfg(not(target_arch = "wasm32"))]
                            start_context_score(
                                &mut audio_engine,
                                MusicContext::GalaxyMap,
                                &mut galaxy_score,
                                player_faction == MissionFaction::Alliance,
                                &sounds_dir,
                                &audio_vol,
                            );
                            #[cfg(target_arch = "wasm32")]
                            start_context_score(
                                browser_menu_audio.as_mut(),
                                MusicContext::GalaxyMap,
                                &mut galaxy_score,
                                player_faction == MissionFaction::Alliance,
                                browser_main_theme.as_deref(),
                                &browser_screen_music,
                            );
                            if let Ok(fingerprint) = restored_fingerprint {
                                macroquad::logging::info!(
                                    "[campaign] restored slot={} mode=Galaxy tick={} fingerprint={} faction={:?}",
                                    slot, clock.tick, fingerprint, player_faction
                                );
                            }
                        }
                        Err(error) => {
                            game_options_state.error = Some(error.to_string());
                        }
                    }
                }
                PanelAction::SetGameSpeed(speed) => choose_game_speed(&mut clock, speed),
                action => {
                    let active_fog_state = if player_faction == MissionFaction::Alliance {
                        &mut fog_alliance_state
                    } else {
                        &mut fog_empire_state
                    };
                    let mut refusal = None;
                    let confirmed = match action {
                        PanelAction::BuildProduction {
                            system,
                            area,
                            count,
                            ..
                        } => Some((system, area, count)),
                        _ => None,
                    };
                    apply_panel_action(
                        action,
                        &mut world,
                        &mut map_state,
                        &mut mfg_state,
                        &mut mission_state,
                        &mut movement_state,
                        &mut troop_transport_state,
                        active_fog_state,
                        &mut ai_state,
                        &mut msg_log,
                        &mut player_faction,
                        &mut clock,
                        &mut dual_ai_mode,
                        &mut secondary_ai_state,
                        &mut victory_state,
                        campaign_config,
                        &mut blockade_state,
                        &event_state,
                        &mut mod_runtime,
                        &message_texts,
                        &mut assault_summary_state,
                        &mut refusal,
                        &mut session_log,
                        session_moment,
                        #[cfg(not(target_arch = "wasm32"))]
                        &mut audio_engine,
                        #[cfg(not(target_arch = "wasm32"))]
                        &audio_vol,
                        #[cfg(not(target_arch = "wasm32"))]
                        &sounds_dir,
                    );
                    // An order's units are objects from its fire
                    // (FUN_0052a140): placed now, in their containers.
                    rebellion_data::player_command::after_order(
                        &mut world,
                        &mfg_state,
                        &troop_transport_state,
                    );
                    if let Some((system, area, count)) = confirmed {
                        system_window_state.confirmed_build(
                            &world, &mfg_state, system, area, count,
                        );
                    }
                    // The advisor reacts to a refused order; without its
                    // messages the port's line stands in.
                    if let Some(refused) = refusal.take() {
                        if let Some(schedule) = advisor_schedule.as_mut() {
                            let today = advisor_now(&clock);
                            schedule.refused(refused.status, refused.decoys_without_target, today);
                            macroquad::logging::info!(
                                "[advisor] reaction refusal status={:#x}/{:#x} decoys={}",
                                refused.status.0,
                                refused.status.1,
                                refused.decoys_without_target
                            );
                        } else {
                            msg_log.push(GameMessage::new(
                                clock.tick,
                                refused.text,
                                MessageCategory::Mission,
                            ));
                        }
                    }
                }
            }
        }

        if quit_requested {
            #[cfg(not(target_arch = "wasm32"))]
            audio_engine.stop_music();
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(engine) = browser_menu_audio.as_mut() {
                    engine.stop_music();
                }
                web_accessibility::sync_menu(false, &main_menu_state, false);
                web_accessibility::show_quit();
            }
            macroquad::logging::info!("[quit] audio_stopped=true cleanup=complete");
            break;
        }

        // 6. Route the recovered tactical score and event cue. The Battle
        // Alert and the Battle Summary play their own original score.
        // A story movie over the summary keeps it open (`screen_score`).
        let screen_music = if game_mode == GameMode::TacticalCombat
            || (matches!(game_mode, GameMode::Galaxy | GameMode::Cutscene { .. })
                && tactical_state.battle_summary_open())
        {
            tactical_state.screen_music()
        } else {
            None
        };
        let tactical_audio_context = game_mode == GameMode::TacticalCombat
            && !tactical_state.battle_alert_open()
            && screen_music.is_none();
        if tactical_audio_context && !tactical_music_active {
            // FUN_00450540: each Battle Alert choice calls FUN_00417640,
            // which stops the score; the tactical battle has none (A0
            // TAC-01-C012: silence while paused, only effects and voices).
            #[cfg(not(target_arch = "wasm32"))]
            audio_engine.stop_music();
            #[cfg(target_arch = "wasm32")]
            if let Some(engine) = browser_menu_audio.as_mut() {
                engine.stop_music();
            }
            macroquad::logging::info!(
                "[audio] context=combat track=none source=FUN_00450540,FUN_00417640 muted={}",
                audio_vol.muted || audio_vol.music_muted
            );
            tactical_music_active = true;
        } else if !tactical_audio_context && tactical_music_active {
            // A battle window's own score follows below.
            if screen_music.is_none() && matches!(game_mode, GameMode::MainMenu | GameMode::Galaxy) {
                let context = if game_mode == GameMode::MainMenu {
                    rebellion_render::MusicContext::MainMenu
                } else {
                    rebellion_render::MusicContext::GalaxyMap
                };
                #[cfg(not(target_arch = "wasm32"))]
                start_context_score(
                    &mut audio_engine,
                    context,
                    &mut galaxy_score,
                    player_faction == MissionFaction::Alliance,
                    &sounds_dir,
                    &audio_vol,
                );
                #[cfg(target_arch = "wasm32")]
                start_context_score(
                    browser_menu_audio.as_mut(),
                    context,
                    &mut galaxy_score,
                    player_faction == MissionFaction::Alliance,
                    browser_main_theme.as_deref(),
                    &browser_screen_music,
                );
            }
            tactical_music_active = false;
        }
        match screen_score(&game_mode, screen_music, &mut screen_music_playing) {
            ScreenScore::Keep => {}
            ScreenScore::Play(track) => {
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_music(track, &sounds_dir, &audio_vol);
                #[cfg(target_arch = "wasm32")]
                if let (Some(engine), Some(bytes)) =
                    (browser_menu_audio.as_mut(), browser_screen_music.get(&track))
                {
                    engine.load_music_bytes(track, bytes);
                }
                macroquad::logging::info!(
                    "[audio] context=battle_window track=original mdata={} loaded={} muted={}",
                    track.mdata().unwrap_or_default(),
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            audio_engine.current_track() == Some(track)
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            browser_screen_music.contains_key(&track)
                        }
                    },
                    audio_vol.muted || audio_vol.music_muted
                );
            }
            ScreenScore::Restore => {
                // Leaving a battle window for the galaxy or the menu
                // restores their score: the galaxy view's next pick.
                let context = if game_mode == GameMode::MainMenu {
                    rebellion_render::MusicContext::MainMenu
                } else {
                    rebellion_render::MusicContext::GalaxyMap
                };
                #[cfg(not(target_arch = "wasm32"))]
                start_context_score(
                    &mut audio_engine,
                    context,
                    &mut galaxy_score,
                    player_faction == MissionFaction::Alliance,
                    &sounds_dir,
                    &audio_vol,
                );
                #[cfg(target_arch = "wasm32")]
                start_context_score(
                    browser_menu_audio.as_mut(),
                    context,
                    &mut galaxy_score,
                    player_faction == MissionFaction::Alliance,
                    browser_main_theme.as_deref(),
                    &browser_screen_music,
                );
            }
        }

        // The advisor's mood (`FUN_00439bf0`) from the systems each side
        // holds, once a game day. hyp: the original refreshes it with the
        // agent's galaxy model; a new game's first pick still sees the
        // constructor's 8 (A0: the Empire's new game began with 303).
        if game_mode == GameMode::Galaxy && galaxy_mood_day != Some(clock.tick) {
            if galaxy_mood_day.is_some() {
                let (own, opponent) =
                    systems_held_by_viewer(&world, player_faction == MissionFaction::Alliance);
                // hyp: agent+0xc4 and +0xc8 (the lead) were not found
                // written anywhere, so they stay equal.
                galaxy_score.refresh_mood(0, own, opponent);
            }
            galaxy_mood_day = Some(clock.tick);
        }
        // In a game the browser keeps the galaxy view's next pick decoding,
        // so the score starts at once when the view shows (a story movie's
        // end, FUN_00422cb0), however soon the next movie follows.
        #[cfg(target_arch = "wasm32")]
        if matches!(
            game_mode,
            GameMode::Galaxy
                | GameMode::Cutscene {
                    kind: CutsceneKind::Story(_)
                }
        ) {
            if let Some(engine) = browser_menu_audio.as_mut().filter(|engine| !engine.has_prepared_music()) {
                let upcoming = galaxy_score.peek(player_faction == MissionFaction::Alliance, |n| {
                    macroquad::rand::rand() % n.max(1)
                });
                if let Some(bytes) = browser_screen_music.get(&upcoming) {
                    engine.prepare_music_bytes(upcoming, bytes);
                }
            }
        }
        // A galaxy pick that ends starts the next (`FUN_00417780`'s
        // 0x4c9 -> `FUN_00417520(FUN_0041d3b0())`; A0: 302 ran out and 301
        // followed).
        #[cfg(not(target_arch = "wasm32"))]
        if audio_engine.galaxy_pick_finished(macroquad::time::get_time()) {
            start_context_score(
                &mut audio_engine,
                MusicContext::GalaxyMap,
                &mut galaxy_score,
                player_faction == MissionFaction::Alliance,
                &sounds_dir,
                &audio_vol,
            );
        }
        #[cfg(target_arch = "wasm32")]
        if browser_menu_audio
            .as_ref()
            .is_some_and(|engine| engine.galaxy_pick_finished(macroquad::time::get_time()))
        {
            start_context_score(
                browser_menu_audio.as_mut(),
                MusicContext::GalaxyMap,
                &mut galaxy_score,
                player_faction == MissionFaction::Alliance,
                browser_main_theme.as_deref(),
                &browser_screen_music,
            );
        }

        // The effects class plays at most five at once; a request waits 225
        // ms for a channel (`FUN_005bae60`, `FUN_005bad50`, `FUN_005bab50`).
        let effect_now = macroquad::time::get_time();
        if tactical_audio_context {
            for cue in tactical_state.take_pending_audio_cues() {
                #[cfg(not(target_arch = "wasm32"))]
                let loaded = audio_engine.has_tactical_sfx(cue.resource_id);
                #[cfg(target_arch = "wasm32")]
                let loaded = browser_menu_audio
                    .as_ref()
                    .is_some_and(|engine| engine.has_tactical_sfx(cue.resource_id));
                macroquad::logging::info!(
                    "[audio] context=combat event=0x{:02x} wave={} routed=true loaded={} muted={}",
                    cue.event.event_id(),
                    cue.resource_id,
                    loaded,
                    audio_vol.muted
                );
                tactical_channels.request(cue, effect_now);
            }
            #[cfg(not(target_arch = "wasm32"))]
            let update = tactical_channels
                .update(effect_now, |wave| audio_engine.tactical_sfx_seconds(wave));
            #[cfg(target_arch = "wasm32")]
            let update = tactical_channels.update(effect_now, |wave| {
                browser_menu_audio
                    .as_ref()
                    .map_or(0.0, |engine| engine.tactical_sfx_seconds(wave))
            });
            for cue in update.started {
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_tactical_sfx(cue.resource_id, &audio_vol);
                #[cfg(target_arch = "wasm32")]
                if let Some(engine) = browser_menu_audio.as_mut() {
                    engine.play_tactical_sfx(cue.resource_id, &audio_vol);
                }
                macroquad::logging::info!(
                    "[audio] context=combat effect=start event=0x{:02x} wave={}",
                    cue.event.event_id(),
                    cue.resource_id
                );
            }
            for cue in update.dropped {
                macroquad::logging::info!(
                    "[audio] context=combat effect=drop event=0x{:02x} wave={}",
                    cue.event.event_id(),
                    cue.resource_id
                );
            }
        } else {
            tactical_channels.clear();
        }
        // The selected unit's hum: a selection state asks it while the
        // battle runs and stops it on pause or when the state ends
        // (`FUN_005a11e0`, `FUN_005a17e0`, `FUN_0059e990`). It takes one
        // of the five channels; with none free it is asked again next frame
        // (`FUN_005baa60` drops a looping request it cannot start).
        let wanted_hum = if tactical_audio_context {
            tactical_state.selection_hum()
        } else {
            None
        };
        if wanted_hum != tactical_hum {
            if let Some(held) = tactical_hum.take() {
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.stop_tactical_loop();
                #[cfg(target_arch = "wasm32")]
                if let Some(engine) = browser_menu_audio.as_mut() {
                    engine.stop_tactical_loop();
                }
                tactical_channels.release_loop();
                macroquad::logging::info!(
                    "[audio] context=combat hum_stop event=0x{:02x} wave={}",
                    held.cue.event.event_id(),
                    held.cue.resource_id
                );
            }
            if let Some(hum) = wanted_hum.filter(|_| tactical_channels.try_hold_loop(effect_now)) {
                #[cfg(not(target_arch = "wasm32"))]
                let loaded = audio_engine.start_tactical_loop(hum.cue.resource_id, &audio_vol);
                #[cfg(target_arch = "wasm32")]
                let loaded = browser_menu_audio.as_mut().is_some_and(|engine| {
                    engine.start_tactical_loop(hum.cue.resource_id, &audio_vol)
                });
                macroquad::logging::info!(
                    "[audio] context=combat hum event=0x{:02x} wave={} unit={:?} looped=true routed=true loaded={} muted={}",
                    hum.cue.event.event_id(),
                    hum.cue.resource_id,
                    hum.unit,
                    loaded,
                    audio_vol.muted
                );
                tactical_hum = Some(hum);
            }
        }
        if tactical_audio_context {
            for cue in tactical_state.take_pending_voice_cues() {
                #[cfg(not(target_arch = "wasm32"))]
                let loaded =
                    audio_engine.play_tactical_voice(cue.faction, cue.resource_id, &audio_vol);
                #[cfg(target_arch = "wasm32")]
                let loaded = browser_menu_audio.as_mut().is_some_and(|engine| {
                    engine.play_tactical_voice(cue.faction, cue.resource_id, &audio_vol)
                });
                macroquad::logging::info!(
                    "[audio] context=combat voice_event={:?} source_event=0x{:x} wave={} faction={:?} routed=true loaded={} muted={}",
                    cue.event,
                    cue.source_event,
                    cue.resource_id,
                    cue.faction,
                    loaded,
                    audio_vol.muted
                );
            }
        }

        // The first gesture (often the click or key that skips the logos)
        // unlocks WebAudio, but the score waits for the screen that plays
        // it: the main menu after the intro, never a movie.
        #[cfg(target_arch = "wasm32")]
        if browser_menu_audio_requested && score_may_start(&game_mode) {
            if let Some(engine) = browser_menu_audio.as_mut() {
                let was_playing = engine.music_is_playing();
                if engine.try_play_loaded_music(&audio_vol) && !was_playing {
                    macroquad::logging::info!(
                        "[audio] score started mode={:?} track={:?}",
                        game_mode,
                        engine.current_track()
                    );
                }
            }
        }

        if persist_settings {
            let settings = current_settings(&audio_vol, &game_options_state);
            if stored_settings != Some(settings) {
                if let Err(error) = rebellion_data::settings::store(&saves_dir, &settings) {
                    macroquad::logging::warn!("[settings] store failed: {}", error);
                }
                stored_settings = Some(settings);
            }
        }

        // Apply audio volume changes on both native and WebAudio backends.
        if audio_vol.dirty {
            #[cfg(not(target_arch = "wasm32"))]
            audio_engine.apply_volume(&audio_vol);
            #[cfg(target_arch = "wasm32")]
            if let Some(engine) = browser_menu_audio.as_mut() {
                engine.apply_volume(&audio_vol);
            }
            macroquad::logging::info!(
                "[audio] music_volume={:.2} muted={} music_muted={}",
                audio_vol.effective_music_volume(),
                audio_vol.muted,
                audio_vol.music_muted
            );
            audio_vol.dirty = false;
        }

        #[cfg(target_arch = "wasm32")]
        {
            if game_mode != GameMode::MainMenu {
                main_menu_state.set_semantic_focus(None);
            }
            web_accessibility::sync_menu(
                game_mode == GameMode::MainMenu,
                &main_menu_state,
                audio_vol.music_enabled(),
            );
        }

        // 7. Handle focus requests from message log + encyclopedia
        if let Some(focus_key) = log_state.focus_system.take() {
            // The map does not pan (galaxy_camera): focusing only selects.
            if world.systems.contains_key(focus_key) {
                map_state.selected_system = Some(focus_key);
            }
        }

        #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
        if let Some(request) = interface_fixture_request {
            if game_mode == GameMode::Galaxy && !interface_fixture_emitted {
                interface_fixture_frames += 1;
                if interface_fixture_frames >= 3 {
                    interface_test_fixture::emit_ready(request, &world, &map_state);
                    interface_test_fixture::emit_fleet_move_setup(
                        request,
                        &world,
                        &sector_window_state,
                        &fleet_window_state,
                    );
                    interface_test_fixture::emit_fleet_load_setup(
                        request,
                        &world,
                        &sector_window_state,
                        &defenses_window_state,
                    );
                    interface_test_fixture::emit_quadrant_setup(
                        request,
                        &world,
                        &sector_window_state,
                    );
                    interface_fixture_emitted = true;
                }
            } else if game_mode == GameMode::Galaxy {
                fleet_move_watch.observe(
                    request,
                    &world,
                    &movement_state,
                    move_confirmation_state.is_open(),
                    &map_state,
                    (
                        &fleet_window_state,
                        if player_faction == MissionFaction::Alliance {
                            &fog_alliance_state
                        } else {
                            &fog_empire_state
                        },
                        &troop_transport_state,
                    ),
                    msg_log.messages(),
                );
                fleet_load_watch.observe(
                    request,
                    &world,
                    if player_faction == MissionFaction::Alliance {
                        &fog_alliance_state
                    } else {
                        &fog_empire_state
                    },
                    &movement_state,
                    &troop_transport_state,
                    &fleet_window_state,
                    move_confirmation_state.is_open(),
                    msg_log.messages(),
                    interface_test_fixture::FixtureAssaultSummary::of(
                        &assault_summary_state,
                        cockpit_state.faction,
                    ),
                    msg_log
                        .queued_story_movies()
                        .find(|movie| movie.movie == 101)
                        .map(|movie| movie.caption.clone()),
                );
                enemy_assault_watch.observe(
                    request,
                    &world,
                    interface_test_fixture::FixtureAssaultSummary::of(
                        &assault_summary_state,
                        cockpit_state.faction,
                    ),
                );
                system_finder_watch.observe(
                    request,
                    &world,
                    &system_finder_state,
                    &sector_window_state,
                );
                fleet_finder_watch.observe(
                    request,
                    &world,
                    &interface_test_fixture::FinderWindows {
                        movement: &movement_state,
                        fog: if player_faction == MissionFaction::Alliance {
                            &fog_alliance_state
                        } else {
                            &fog_empire_state
                        },
                        finder: &fleet_finder_state,
                        fleets: &fleet_window_state,
                        sectors: &sector_window_state,
                        map: &map_state,
                        // No replacement side panel remains; the Fleet
                        // Finder probe still checks no letter opens one.
                        left_panel_open: false,
                    },
                );
                quadrant_watch.observe(
                    request,
                    &world,
                    &interface_test_fixture::QuadrantWindows {
                        systems: &system_window_state,
                        defenses: &defenses_window_state,
                        missions_windows: &missions_window_state,
                        fog: if player_faction == MissionFaction::Alliance {
                            &fog_alliance_state
                        } else {
                            &fog_empire_state
                        },
                        missions: &mission_state,
                        production: rebellion_render::DefensesProduction {
                            manufacturing: &mfg_state,
                            deliveries: &delivery_state,
                            regiments: troop_transport_state.transits(),
                            squadrons: troop_transport_state.squadrons.transits(),
                        },
                        economy: &economy_state,
                    },
                );
            } else if game_mode == GameMode::TacticalCombat {
                // The battle variant follows the planet's squadrons into
                // the battle the clock starts.
                fleet_load_watch.observe(
                    request,
                    &world,
                    if player_faction == MissionFaction::Alliance {
                        &fog_alliance_state
                    } else {
                        &fog_empire_state
                    },
                    &movement_state,
                    &troop_transport_state,
                    &fleet_window_state,
                    move_confirmation_state.is_open(),
                    msg_log.messages(),
                    interface_test_fixture::FixtureAssaultSummary::of(
                        &assault_summary_state,
                        cockpit_state.faction,
                    ),
                    msg_log
                        .queued_story_movies()
                        .find(|movie| movie.movie == 101)
                        .map(|movie| movie.caption.clone()),
                );
            }
        }
        #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
        if let Some(request) = tactical_fixture_request {
            // A fixture that opens on the Battle Summary is ready over the
            // Command Center.
            if (game_mode == GameMode::TacticalCombat || tactical_state.battle_summary_open())
                && !interface_fixture_emitted
            {
                interface_fixture_frames += 1;
                if interface_fixture_frames >= 3 {
                    tactical_test_fixture::emit_ready(request, &tactical_state);
                    interface_fixture_emitted = true;
                }
            }
        }

        if targeting_cursor_drawn != system_cursor_hidden {
            show_mouse(!targeting_cursor_drawn);
            system_cursor_hidden = targeting_cursor_drawn;
        }

        // The strategy view as it stands, for the Battle Alert and Battle
        // Results windows the battle opens over it (A0 battle-clean/00,
        // message-rail/battle-averam-result). A copy on the GPU, made once
        // the frame's batches are drawn, in the frame the battle began in;
        // the battle then takes the screen. A frame that left the galaxy
        // meanwhile (Game Options from the cockpit) yields to the battle.
        if battle_underlay_pending
            && matches!(
                game_mode,
                GameMode::Galaxy
                    | GameMode::GameOptions {
                        origin: GameOptionsOrigin::CommandCenter
                    }
            )
        {
            if game_mode == GameMode::Galaxy {
                let (width, height) = (screen_width(), screen_height());
                let fits = tactical_state
                    .strategy_underlay
                    .as_ref()
                    .is_some_and(|texture| texture.width() == width && texture.height() == height);
                if !fits {
                    tactical_state.strategy_underlay =
                        Some(render_target(width as u32, height as u32).texture);
                }
                if let Some(texture) = tactical_state.strategy_underlay.as_ref() {
                    unsafe {
                        get_internal_gl().flush();
                    }
                    texture.grab_screen();
                }
            } else {
                // No galaxy was drawn: the windows fall back to the plain
                // backdrop rather than an older battle's copy.
                tactical_state.strategy_underlay = None;
            }
            game_mode = GameMode::TacticalCombat;
        }

        next_frame().await;
    }
}

// ---------------------------------------------------------------------------
// Panel action handler
// ---------------------------------------------------------------------------

#[cfg(test)]
mod advisor_event_tests {
    use super::{blockade_event, joins_event, uprising_event};
    use rebellion_core::dat::Faction;

    #[test]
    fn a_system_joining_one_side_is_1_to_it_and_2_to_the_other() {
        // FUN_00499760: +0x28 = 1 when the new side is the reader, else 2;
        // neutrality writes none.
        assert_eq!(joins_event(Some(Faction::Alliance), Faction::Alliance), 1);
        assert_eq!(joins_event(Some(Faction::Alliance), Faction::Empire), 2);
        assert_eq!(joins_event(Some(Faction::Empire), Faction::Empire), 1);
        assert_eq!(joins_event(Some(Faction::Neutral), Faction::Empire), 0);
        assert_eq!(joins_event(None, Faction::Alliance), 0);
    }

    #[test]
    fn an_uprising_is_1_when_holding_differs_from_beginning() {
        // FUN_00499460: local_4 (reader holds) against +100 (begins).
        let held = Some(Faction::Empire);
        assert_eq!(uprising_event(held, Faction::Empire, true), 2);
        assert_eq!(uprising_event(held, Faction::Alliance, true), 1);
        assert_eq!(uprising_event(held, Faction::Empire, false), 1);
        assert_eq!(uprising_event(held, Faction::Alliance, false), 2);
        assert_eq!(uprising_event(Some(Faction::Neutral), Faction::Alliance, true), 0);
        assert_eq!(uprising_event(None, Faction::Alliance, false), 0);
    }

    #[test]
    fn a_blockade_is_0xe_to_the_holder_and_0xd_to_the_blockader() {
        // FUN_004960f0.
        assert_eq!(blockade_event(Some(Faction::Alliance), Faction::Alliance), 0xe);
        assert_eq!(blockade_event(Some(Faction::Alliance), Faction::Empire), 0xd);
    }
}

#[cfg(test)]
mod advisor_now_tests {
    use super::advisor_now;
    use rebellion_core::tick::{GameClock, GameSpeed};

    #[test]
    fn the_advisor_counts_sub_ticks_from_one() {
        // DAT_006b28cc is FUN_004fcee0's sub-tick (FUN_0051dea0): 13 a day
        // at Medium, the rate 12 and the rollover; hyp: the count starts at 1.
        let mut clock = GameClock::new();
        assert_eq!(advisor_now(&clock), 1);
        clock.set_speed(GameSpeed::Medium);
        let day = GameSpeed::Medium.seconds_per_day().unwrap();
        assert_eq!(clock.advance(day * 1.01).len(), 1);
        assert_eq!(advisor_now(&clock), 14);
    }
}

#[cfg(test)]
mod settings_tests {
    use super::{apply_settings, current_settings, AudioVolumeState, GameOptionsState};
    use rebellion_data::settings::display;

    #[test]
    fn the_game_options_round_trip_through_their_stored_settings() {
        // FUN_004082b0 masks; FUN_004173c0 reads them back at start-up.
        let mut audio = AudioVolumeState {
            music_volume: 0.49,
            sfx_volume: 0.2,
            music_muted: true,
            ..AudioVolumeState::default()
        };
        let mut options = GameOptionsState::default();
        options.show_planet = false;
        options.display_holocube = false;
        let stored = current_settings(&audio, &options);
        assert_eq!(
            stored.display_options,
            display::STARFIELD | display::PYROTECHNICS | display::HIGH_DETAIL
        );
        assert_eq!((stored.music_volume, stored.effects_volume), (0.49, 0.2));
        let (mut restored_audio, mut restored) =
            (AudioVolumeState::default(), GameOptionsState::default());
        apply_settings(&stored, &mut restored_audio, &mut restored);
        assert_eq!(current_settings(&restored_audio, &restored), stored);
        assert!(restored_audio.music_muted && restored_audio.dirty);
        audio.music_muted = false;
        options.show_planet = true;
        let flipped = current_settings(&audio, &options);
        assert!(flipped.shows(display::MUSIC) && flipped.shows(display::PLANET));
    }
}

#[cfg(test)]
mod refusal_status_tests {
    use super::refusal_status;
    use rebellion_core::missions::{MissionMember, MissionRefusal};

    #[test]
    fn only_an_emptied_team_with_decoys_and_no_target_takes_the_decoy_reaction() {
        // FUN_004c2940 / FUN_004c0870 (mission-dialog.md, "Refusal").
        let empty = MissionRefusal::EmptyTeam;
        assert_eq!(refusal_status(empty, false, true), ((0x40, 0x91), true));
        assert_eq!(refusal_status(empty, true, true), ((0x40, 0x91), false));
        assert_eq!(refusal_status(empty, false, false), ((0x40, 0x91), false));
        let member = MissionMember::Character(Default::default());
        assert_eq!(
            refusal_status(MissionRefusal::MemberUnavailable(member), false, true),
            ((0x40, 0x91), false)
        );
        assert_eq!(
            refusal_status(MissionRefusal::TargetUnavailable, false, true),
            ((0x40, 0x28), false)
        );
        assert_eq!(
            refusal_status(MissionRefusal::MembersNotAllowed, false, true),
            ((0x40, 0x01), false)
        );
    }
}

// ---------------------------------------------------------------------------
// Effect application helpers
// ---------------------------------------------------------------------------

/// The 640 by 480 client the original's movie window fills
/// (`FUN_005f6920` centres the movie in it), scaled into the screen:
/// `(x, y, scale)`.
fn movie_client() -> (f32, f32, f32) {
    let scale = (screen_width() / 640.0).min(screen_height() / 480.0);
    (
        (screen_width() - 640.0 * scale) * 0.5,
        (screen_height() - 480.0 * scale) * 0.5,
        scale,
    )
}

fn draw_fullscreen_texture(texture: &Texture2D) {
    let texture_width = texture.width();
    let texture_height = texture.height();
    if texture_width <= 0.0 || texture_height <= 0.0 {
        return;
    }

    let (client_x, client_y, client_scale) = movie_client();
    let scale = (640.0 * client_scale / texture_width).min(480.0 * client_scale / texture_height);
    let dest_width = texture_width * scale;
    let dest_height = texture_height * scale;
    let x = client_x + (640.0 * client_scale - dest_width) * 0.5;
    let y = client_y + (480.0 * client_scale - dest_height) * 0.5;

    draw_texture_ex(
        texture,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(dest_width, dest_height)),
            ..Default::default()
        },
    );
}

/// A story movie's caption (`FUN_0041ecf0`, class "TextScrollField"): 580
/// by 70 at (4, 404) in the movie's 640 by 480 client, under the
/// letterboxed movie (rows 78..401). White font 4 on black, from its top
/// left, wrapped by words at 580 (`FUN_0041fd00`, `FUN_0041fe70`); lines
/// past its 70 rows are not drawn. port: the scroll bar the field adds to
/// a longer text is not drawn; no shipped caption needs one.
fn draw_story_caption(caption: &str) {
    use egui_macroquad::egui;
    let (x, y, scale) = movie_client();
    let rect = egui::Rect::from_min_size(
        egui::pos2(x + 4.0 * scale, y + 404.0 * scale),
        egui::vec2(580.0 * scale, 70.0 * scale),
    );
    egui_macroquad::ui(|ctx| {
        let painter = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("story-caption"),
            ))
            .with_clip_rect(rect);
        // DrawTextA with DT_WORDBREAK (theme::paint_gdi_text_block).
        let _ = rebellion_render::theme::paint_gdi_text_block(
            &painter,
            rect,
            scale,
            4,
            caption,
            egui::Color32::WHITE,
            rebellion_render::theme::GdiAlign::Left,
        );
    });
    egui_macroquad::draw();
}

const fn trench_run_cutscene_path(outcome: TacticalTrenchRunOutcome) -> &'static str {
    match outcome {
        TacticalTrenchRunOutcome::Success => TRENCH_RUN_SUCCESS_CUTSCENE,
        TacticalTrenchRunOutcome::Failure => TRENCH_RUN_FAILURE_CUTSCENE,
    }
}

/// Whether an `OPEN_REBELLION_MUTE` or `OPEN_REBELLION_DEV` value turns its
/// switch on: 1, true, yes or on, in any case.
#[cfg(not(target_arch = "wasm32"))]
fn env_flag_on(value: &str) -> bool {
    ["1", "true", "yes", "on"]
        .iter()
        .any(|on| value.trim().eq_ignore_ascii_case(on))
}

/// Start a movie. One whose file cannot be opened shows the video
/// manager's error box instead and ends when it is answered
/// (`FUN_005f6920`); the original logs nothing to the Message Index.
fn open_cutscene(
    path: &Path,
    volume: f32,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
    #[cfg(target_arch = "wasm32")] browser_audio: Option<&mut audio::AudioEngine>,
) -> Option<VideoPlayer> {
    // Every movie stops the score first (`FUN_00610c00(0)`: the credits'
    // command 0x73 in `FUN_00406000`, a story movie's 0x482 in
    // `FUN_00422ce0`, the trench run's `FUN_005cfec0`).
    #[cfg(not(target_arch = "wasm32"))]
    audio_engine.stop_music();
    #[cfg(target_arch = "wasm32")]
    if let Some(engine) = browser_audio {
        engine.halt_music();
    }

    match VideoPlayer::open(path, volume) {
        Ok(player) => {
            macroquad::logging::info!("[cutscene] opened path={}", path.display());
            Some(player)
        }
        Err(error) => {
            if matches!(error, VideoError::NotDecoded { .. }) {
                eprintln!("[cutscene] {error} — run scripts/decode-cutscenes.sh");
            } else {
                eprintln!("[cutscene] {error}");
            }
            Some(VideoPlayer::unopened(path))
        }
    }
}

/// The video manager's error box over the movie window (`FUN_0060a9e0`):
/// paint it and answer its keys and clicks; OK ends the movie (0x403).
fn answer_movie_error_box(player: &mut VideoPlayer) {
    use egui_macroquad::egui;
    use rebellion_render::movie_error_box::BoxInput;
    let (x, y, scale) = movie_client();
    let Some(dialog) = player.error_box_mut() else {
        return;
    };
    let mut layout = None;
    egui_macroquad::ui(|ctx| {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("movie-error-box"),
        ));
        layout = Some(dialog.paint(&painter, egui::pos2(x, y), scale));
    });
    egui_macroquad::draw();
    let Some(layout) = layout else {
        return;
    };
    let (mouse_x, mouse_y) = mouse_position();
    let (client_x, client_y) = ((mouse_x - x) / scale, (mouse_y - y) / scale);
    let mut inputs = Vec::new();
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        inputs.push(BoxInput::EnterPressed);
    }
    if is_key_pressed(KeyCode::Escape) {
        inputs.push(BoxInput::EscapePressed);
    }
    if is_key_released(KeyCode::Space) {
        inputs.push(BoxInput::SpaceReleased);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        inputs.push(BoxInput::MouseDown(client_x, client_y));
    }
    if is_mouse_button_released(MouseButton::Left) {
        inputs.push(BoxInput::MouseUp(client_x, client_y));
    }
    // In order: a press and a release in one frame make a click.
    if inputs
        .into_iter()
        .any(|input| dialog.handle(input, &layout))
    {
        player.close_error_box();
    }
}

/// The mode a frame runs in: once a battle has ended and its Battle Summary
/// is open, the Command Center's. The original creates the summary after
/// the tactical module returns, as a child window of the Command Center
/// (`FUN_00488b90` -> `FUN_0041d730` -> `FUN_0042a640` on `DAT_006b14cc`,
/// once `FUN_0042d490` finds that window visible).
fn battle_summary_mode(mode: &GameMode, summary_open: bool) -> GameMode {
    if summary_open && *mode == GameMode::TacticalCombat {
        GameMode::Galaxy
    } else {
        mode.clone()
    }
}

/// Whether the galaxy's clock runs this frame: in the Command Center, and
/// not while a battle's Battle Summary is open. `FUN_0040a050`'s state 3
/// sets the game's `+0x14`, which stops the day step (`FUN_0041e390` calls
/// `FUN_0051dea0` only while it is 0), until state `0xc` clears it
/// (`FUN_0041e320`). State `0xb` waits there until both players are done
/// (`FUN_00436400`: `+0x38` = 3), which the summary's Close sets
/// (`FUN_00470ea0` -> `FUN_0041ce80` -> `FUN_00487890`).
///
/// Nor once the victory conditions word says the game is over: the gate
/// `FUN_0051de80` reads the scheduler's galaxy (`+0xc0`, a `FUN_00513240`
/// object) word `+0x74` for `0x10000000`, and while it is set
/// `FUN_0041e390` skips the sub-tick and `FUN_0051dea0` raises neither the
/// sub-ticks `+0xc` nor the day `+0x10`. The advisor's victory speech
/// (`FUN_004c0f10` / `FUN_004c3010`) then plays over a held galaxy.
/// Whether the advisor's update runs this frame. `FUN_0041cea0` gates it on
/// `DAT_006b14e4`, the 0x34-byte advisor object `FUN_00435a50` /
/// `FUN_00435ad0` build for the game, not on the galaxy view; a story movie
/// plays on Smacker's own thread (`FUN_005f7020`) while the app loop keeps
/// calling the frame scheduler (`FUN_00413560` -> `FUN_00401bd0`), so the
/// advisor's clips and speech go on under it. An ending requested then
/// (`0x482(0, state)`) waits in `FUN_00422ce0` while the movie holds the
/// capture (`FUN_00601340`) and starts at its end. A0 2026-11-17
/// edited-saves-5 h-07..h-09 (END-01-C001): one strategy frame, C-3PO
/// silent, between MDATA.103's last frame and MDATA.105.
fn advisor_runs(mode: &GameMode) -> bool {
    matches!(
        mode,
        GameMode::Galaxy
            | GameMode::Cutscene {
                kind: CutsceneKind::Story(_)
            }
    )
}

/// The session check's line: the log's replay against the game's state
/// `live` at `moment` (rebellion_data::session).
fn session_check(
    log: &rebellion_data::session::SessionLog,
    live: &rebellion_data::save::SaveState,
    moment: rebellion_data::session::OrderMoment,
) -> String {
    if !log.has_start() {
        return "Session replay: no session start yet".to_string();
    }
    let orders = log.entries().len();
    let unrecorded = log.unrecorded();
    let replayed = log
        .replay(live.clock.tick, moment)
        .and_then(|runner| runner.fingerprint());
    let game = rebellion_data::save::compute_state_fingerprint(live);
    match (replayed, game) {
        (Ok(replayed), Ok(game)) if replayed == game => format!(
            "Session replay matches the game: {orders} commands, day {}, {} unrecorded",
            live.clock.tick, unrecorded
        ),
        (Ok(replayed), Ok(game)) => format!(
            "Session replay differs: {orders} commands, day {}, {} unrecorded, replay {} game {}",
            live.clock.tick, unrecorded, replayed.value, game.value
        ),
        (Err(error), _) | (_, Err(error)) => format!("Session replay failed: {error}"),
    }
}

/// Where in the game's time an order given now falls: in the open day after
/// its passes so far, or between days (rebellion_data::session).
fn order_moment(ai_day: &AiDay) -> rebellion_data::session::OrderMoment {
    if ai_day.loyalty_before.is_some() {
        rebellion_data::session::OrderMoment::InDay {
            passes_done: ai_day.done,
        }
    } else {
        rebellion_data::session::OrderMoment::BetweenDays
    }
}

fn galaxy_clock_runs(mode: &GameMode, summary_open: bool, conditions_word: u32) -> bool {
    *mode == GameMode::Galaxy
        && !summary_open
        && conditions_word & rebellion_core::victory::WORD_GAME_OVER == 0
}

/// The Battle Summary's destinations (`FUN_00470ea0`, vtable `0x65a300`):
/// the system button `0xcb` finds the battle's system (`FUN_004f3220`) and
/// opens its sector window (`FUN_00429ce0`, 235 by 360); the fleet button
/// `0xca` opens the sector window too, then the Fleet window
/// (`FUN_0045c8e0`, kind 4). A0 TAC-07-C014: the Sluis sector window
/// after the Battle at Mon Calamari.
fn open_battle_result_sector(
    world: &GameWorld,
    system: SystemKey,
    sector_windows: &mut SectorWindowState,
    faction: CockpitFaction,
) -> bool {
    sector_windows.open_for_system(world, system, faction)
}

#[cfg(test)]
#[path = "main_tests/tactical_ground_tests.rs"]
mod tactical_ground_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "main_tests/options_save_tests.rs"]
mod options_save_tests;

#[cfg(test)]
#[path = "main_tests/fleet_move_tests.rs"]
mod fleet_move_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod launch_mute_tests {
    use super::env_flag_on;

    #[test]
    fn the_mute_variable_takes_common_true_words_in_any_case() {
        for on in ["1", "true", "TRUE", "Yes", "on", " on\n"] {
            assert!(env_flag_on(on), "{on:?}");
        }
        for off in ["", "0", "false", "no", "off", "muted"] {
            assert!(!env_flag_on(off), "{off:?}");
        }
    }
}

#[cfg(test)]
#[path = "main_tests/character_health_tests.rs"]
mod character_health_tests;

#[cfg(test)]
#[path = "main_tests/assault_report_tests.rs"]
mod assault_report_tests;
#[cfg(test)]
#[path = "main_tests/a0_message_text_tests.rs"]
mod a0_message_text_tests;

#[cfg(test)]
#[path = "main_tests/shared_day_tests.rs"]
mod shared_day_tests;
