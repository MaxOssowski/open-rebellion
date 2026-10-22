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

use rebellion_core::ai::{AIAction, AIState, AISystem, AiFaction, FleetMoveReason};
use rebellion_core::blockade::{BlockadeState, BlockadeSystem};
use rebellion_core::bombardment::BombardEnv;
use rebellion_core::combat::CombatSide;
use rebellion_core::agent_automation::{AutomationModule, PlayerAgent};
use rebellion_core::dat::Faction;
use rebellion_core::delivery::DeliveryState;
use rebellion_core::economy::{EconomyEvent, EconomyState, EconomySystem};
use rebellion_core::events::{EventState, EventSystem};
use rebellion_core::fleet_join::FleetMover;
use rebellion_core::fog::{FogState, FogSystem};
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey};
use rebellion_core::manufacturing::{
    ManufacturingState, QueueItem,
};
use rebellion_core::missions::{
    MissionEffect, MissionFaction, MissionKind, MissionMember, MissionState, MissionSystem,
};
use rebellion_core::movement::{
    apply_fleet_arrival, begin_faction_fleet_transit, begin_fleet_transit, fleet_move_confirms,
    fleet_move_enabled, fleet_transit_ticks, fleets_move_enabled, reconcile_fleet_orbits,
    system_side_fleets, validate_fleet_dispatch,
    validate_fleets_dispatch, MovementState, MovementSystem,
};
use rebellion_core::production_cycle::{
    advance_yards, scrap_yardless_queues, suspended_systems, ProductionState, ProductionSystem,
};
use rebellion_core::repair::{RepairEvent, RepairState, RepairSystem};
use rebellion_core::research::ResearchState;
use rebellion_core::tick::{GameClock, GameSpeed};
use rebellion_core::squadrons::SquadronPlace;
use rebellion_core::troop_transport::{RegimentLeg, RegimentTarget, TroopTransportState};
use rebellion_core::uprising::{UprisingState, UprisingSystem};
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
    MoveConfirmationState,
};
use rebellion_render::object_menu::{
    draw_object_menu, MenuObject, ObjectMenuCommand, ObjectMenuState, OrderGates,
};
use rebellion_render::quadrant_icons::Quadrant;
use rebellion_render::status_rows::StatusSources;
use rebellion_render::status_window::{draw_status_window, StatusWindowAction, StatusWindowState};
use rebellion_render::system_window::fleet_label;
use rebellion_render::targeting::{
    capture_pointer, draw_targeting_cursor, mission_release_target, release_destination,
    MissionReleaseSources, ReleaseTarget, ReleaseWindows,
    TargetOrder, Targeting, TargetingEnd,
};
use rebellion_render::{
    advisor_combat_result, advisor_greet, advisor_manufacturing_complete,
    advisor_mission_result, advisor_uprising, draw_advisor, draw_audio_controls,
    draw_cockpit_background, draw_cockpit_chrome, draw_cockpit_egui_layer, draw_credits,
    draw_encyclopedia, draw_galaxy_backdrop, draw_galaxy_map, draw_game_options,
    draw_game_setup, draw_gid_caption, draw_main_menu, draw_multiplayer_setup,
    draw_save_load, draw_sector_windows, draw_system_windows, draw_tactical_view,
    handle_cockpit_egui_input, set_cockpit_viewport_clip, AdvisorFaction, AdvisorState,
    AssetRenderProfile, AudioVolumeState, BmpCache, CockpitButton, CockpitFaction, CockpitState,
    CreditsState, EncyclopediaState, GalaxyMapState, GameMessage, GameOptionsAction,
    GameOptionsOrigin, GameOptionsState, GameSetupAction, GameSetupState, MainMenuAction,
    MainMenuState, MenuDestinationAction, MessageCategory, MessageLog, MessageLogState,
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
    /// Save-slot picker entered from the main menu.
    LoadGame,
    /// Scrolling original-game and Open Rebellion credits.
    Credits,
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
    /// Victory/defeat modal overlay on the frozen galaxy map.
    VictoryModal { alliance_won: bool },
}

/// Which cutscene is playing — determines post-cutscene transition.
#[derive(Debug, Clone, PartialEq)]
enum CutsceneKind {
    /// Game intro (000.webm) → `MainMenu`.
    Intro,
    /// Death Star trench-run result (201/202.webm) → resume tactical view.
    TrenchRun(TacticalTrenchRunOutcome),
    /// In-game story cutscene (101–108.webm) → resume Galaxy.
    Story(u32),
}

const INTRO_CUTSCENE: &str = "assets/references/ref-videos/000.webm";
const TRENCH_RUN_SUCCESS_CUTSCENE: &str = "assets/references/ref-videos/201.webm";
const TRENCH_RUN_FAILURE_CUTSCENE: &str = "assets/references/ref-videos/202.webm";

/// Map a story event ID to its cutscene file number (101–108), if any.
/// Returns `None` for events that don't trigger a cutscene.
fn story_event_to_cutscene(event_id: u32) -> Option<u32> {
    match event_id {
        0x221 => Some(101), // Luke departs for Dagobah
        0x210 => Some(102), // Luke completes Dagobah training
        0x212 => Some(103), // Bounty hunters capture Han
        0x383 => Some(104), // Palace rescue success
        0x220 => Some(105), // Luke vs Vader final confrontation
        0x393 => Some(106), // Vader dispatched
        0x396 => Some(107), // Father and son confrontation
        0x397 => Some(108), // Empire dispatches bounty hunters
        _ => None,
    }
}

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
/// cutscene stops the music, so its end has to start this again.
#[cfg(not(target_arch = "wasm32"))]
fn music_context_after_cutscene(kind: &CutsceneKind) -> MusicContext {
    match kind {
        CutsceneKind::Intro => MusicContext::MainMenu,
        CutsceneKind::TrenchRun(_) => MusicContext::Combat,
        CutsceneKind::Story(_) => MusicContext::GalaxyMap,
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
mod audio_source_tests {
    use super::*;

    #[test]
    fn an_original_dll_is_read_from_the_installation_before_the_gdata_directory() {
        let root =
            std::env::temp_dir().join(format!("open-rebellion-dll-path-{}", std::process::id()));
        let game_dir = root.join("install");
        let gdata_path = root.join("gdata");
        std::fs::create_dir_all(&game_dir).expect("create install dir");
        std::fs::create_dir_all(&gdata_path).expect("create gdata dir");

        // Staged beside the DAT files only: the GData copy is the source.
        std::fs::write(gdata_path.join("TACTICAL.DLL"), b"gdata").expect("write gdata dll");
        assert_eq!(
            original_dll_path(&game_dir, &gdata_path, "TACTICAL.DLL"),
            gdata_path.join("TACTICAL.DLL")
        );

        std::fs::write(game_dir.join("TACTICAL.DLL"), b"install").expect("write installed dll");
        assert_eq!(
            original_dll_path(&game_dir, &gdata_path, "TACTICAL.DLL"),
            game_dir.join("TACTICAL.DLL")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_finished_cutscene_restarts_the_score_of_the_mode_it_returns_to() {
        assert_eq!(
            music_context_after_cutscene(&CutsceneKind::Intro),
            MusicContext::MainMenu
        );
        assert_eq!(
            music_context_after_cutscene(&CutsceneKind::Story(101)),
            MusicContext::GalaxyMap
        );
        for outcome in [
            TacticalTrenchRunOutcome::Success,
            TacticalTrenchRunOutcome::Failure,
        ] {
            assert_eq!(
                music_context_after_cutscene(&CutsceneKind::TrenchRun(outcome)),
                MusicContext::Combat
            );
        }
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

fn read_save_slots(saves_dir: &Path) -> Vec<rebellion_render::SaveSlotInfo> {
    let mut slots: Vec<_> = rebellion_data::save::list_saves(saves_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
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
        })
        .collect();
    for slot in 0..rebellion_data::save::MAX_SAVE_SLOTS {
        if !slots.iter().any(|save| save.slot == slot)
            && rebellion_data::save::slot_occupied(saves_dir, slot)
        {
            slots.push(rebellion_render::SaveSlotInfo {
                slot,
                name: "Unreadable save".into(),
                timestamp: "Load to inspect error".into(),
                game_tick: 0,
            });
        }
    }
    slots.sort_by_key(|save| save.slot);
    slots
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
    game_config: &'a mut rebellion_core::tuning::GameConfig,
    campaign_config: &'a mut CampaignConfig,
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
            game_config: self.game_config.clone(),
            campaign_config: *self.campaign_config,
            troop_transport: self.troop_transport.clone(),
            deliveries: self.deliveries.clone(),
            player_agent: self.player_agent.clone(),
        }
    }

    fn restore(&mut self, state: rebellion_data::save::SaveState) {
        *self.world = state.world;
        *self.clock = state.clock;
        *self.manufacturing = state.manufacturing;
        *self.missions = state.missions;
        *self.events = state.events;
        *self.ai = state.ai;
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
        *self.game_config = state.game_config;
        *self.campaign_config = state.campaign_config;
        *self.troop_transport = state.troop_transport;
        *self.deliveries = state.deliveries;
        *self.player_agent = state.player_agent;
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
    bytes: &[u8],
) -> Result<std::collections::HashMap<String, Vec<u8>>, String> {
    let mut pack = runtime_pack::parse_runtime_pack(bytes).map_err(|error| error.to_string())?;
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
        .filter(|(key, _)| key.starts_with("alsprite-dll/") || key.starts_with("emsprite-dll/"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    if let Some(legends) = pack.game_files.remove("gid-legends.json") {
        rebellion_data::set_gid_legends_json(&legends);
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
        bytes.len()
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
        Ok(bytes) => install_runtime_pack(&bytes)
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
    // The galaxy view's legend windows (STRATEGY.DLL RCDATA, FUN_004522f0).
    rebellion_render::set_gid_legends(rebellion_data::gid_legends(&gdata_path));

    // The Galactic Encyclopedia is immutable reference data, not campaign
    // state. Rebuild it from the installed DAT/TEXTSTRA source so opening the
    // index after a save/load never changes the serialized world format.
    let original_encyclopedia_catalog =
        match rebellion_data::encyclopedia_catalog::load_encyclopedia_catalog(&gdata_path) {
            Ok(source) => {
                let category_counts: [usize; 7] = std::array::from_fn(|index| {
                    source
                        .entries_for(source.categories[index].command_id)
                        .len()
                });
                let category_labels = source.categories.map(|category| category.label);
                let entries = source
                    .entries
                    .into_iter()
                    .map(|entry| OriginalEncyclopediaEntry {
                        object_id: entry.object_id,
                        name: entry.name,
                    })
                    .collect();
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
                    "[encyclopedia] source_catalog unavailable error={error}"
                );
                None
            }
        };
    #[cfg(not(all(target_arch = "wasm32", feature = "interface-test-fixtures")))]
    let _ = original_encyclopedia_catalog.as_ref();

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
    let mut credits_state = CreditsState::default();
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
    let mut game_config = rebellion_core::tuning::GameConfig::default();
    let mut campaign_config = CampaignConfig::default();
    let mut dual_ai_mode = false;
    let mut secondary_ai_state: Option<AIState> = None;
    let mut movement_state = MovementState::new();
    let mut fog_alliance_state = FogState::new(Faction::Alliance);
    let mut fog_empire_state = FogState::new(Faction::Empire);
    FogSystem::seed(&mut fog_alliance_state, &world);
    FogSystem::seed(&mut fog_empire_state, &world);
    // The planet squadrons up for the player's battle, landed when it ends.
    let mut battle_sorties: Vec<rebellion_core::squadrons::Sortie> = Vec::new();
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
    let mut show_save_load = false;
    let mut save_load_panel_state = rebellion_render::SaveLoadPanelState::default();
    let saves_dir = rebellion_data::save::default_saves_dir();
    let mut save_slots = read_save_slots(&saves_dir);

    // ── Tactical combat state ────────────────────────────────────────────────
    let mut tactical_state = TacticalState::new();

    // ── Cockpit chrome ───────────────────────────────────────────────────────
    let mut cockpit_state = CockpitState::new(CockpitFaction::Alliance);
    let mut game_speed_ui = GameSpeedUiState::default();
    let mut sector_window_state = SectorWindowState::default();
    let mut system_window_state = SystemWindowState::default();
    let mut fleet_window_state = FleetWindowState::default();
    let mut defenses_window_state = DefensesWindowState::default();
    let mut assault_summary_state =
        rebellion_render::assault_summary::AssaultSummaryState::default();
    let mut missions_window_state = MissionsWindowState::default();
    // The object pop-up menu a right-click in a system window opens.
    let mut object_menu: Option<ObjectMenuState> = None;
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
    #[cfg(target_arch = "wasm32")]
    let browser_battle_theme = browser_audio_files.remove("music/battle.wav");
    // The battle windows' original score (FUN_0044f860, FUN_0046f360).
    #[cfg(target_arch = "wasm32")]
    let browser_screen_music: std::collections::HashMap<rebellion_render::MusicTrack, Vec<u8>> = (10..=15)
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
    #[cfg(target_arch = "wasm32")]
    let mut browser_menu_audio = if browser_main_theme.is_some()
        || browser_battle_theme.is_some()
        || !browser_menu_sfx.is_empty()
        || !browser_tactical_sfx.is_empty()
        || !browser_tactical_voice.is_empty()
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
    let mut screen_music_playing: Option<rebellion_render::MusicTrack> = None;

    let mut cutscene_player = open_cutscene(
        Path::new(INTRO_CUTSCENE),
        audio_vol.cutscene_volume(),
        &mut msg_log,
        clock.tick,
        #[cfg(not(target_arch = "wasm32"))]
        &mut audio_engine,
    );
    if cutscene_player.is_some() {
        game_mode = GameMode::Cutscene {
            kind: CutsceneKind::Intro,
        };
    }

    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let interface_fixture_request = interface_test_fixture::requested();
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
        if interface_test_fixture::runs_the_clock(request) {
            choose_game_speed(&mut clock, GameSpeed::Fast);
        }
        interface_test_fixture::reveal(
            request,
            &world,
            &mut fog_alliance_state,
            &mut fog_empire_state,
        );
        interface_test_fixture::post_story_messages(request, &world, &mut msg_log, &message_texts);
        interface_test_fixture::post_event_messages(request, &world, &mut msg_log);
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
            &mut tactical_state,
            &mut combat_cooldowns,
            &mut msg_log,
            &mut player_faction,
            &mut cockpit_state,
            &mut game_mode,
        ) {
            Ok(()) => false,
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
    let mut speed_menu_watch = interface_test_fixture::SpeedMenuWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut quadrant_watch = interface_test_fixture::QuadrantWatch::default();
    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
    let mut fleet_finder_watch = interface_test_fixture::FleetFinderWatch::default();
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

        // ── Advisor animation timer ────────────────────────────────────────
        if !interface_fixture_active {
            advisor_state.update(dt);
        }

        // ── Global keyboard shortcuts ───────────────────────────────────────
        if matches!(game_mode, GameMode::Cutscene { .. }) {
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Space) {
                if let Some(player) = cutscene_player.as_mut() {
                    player.stop();
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            if let GameMode::GameOptions { origin } = game_mode.clone() {
                if game_options_state.suspended {
                    save_load_panel_state.close();
                    game_options_state.suspended = false;
                } else if game_options_state.escape() == GameOptionsAction::Return {
                    game_mode = match origin {
                        GameOptionsOrigin::ShuttleCockpit => GameMode::MainMenu,
                        GameOptionsOrigin::CommandCenter => GameMode::Galaxy,
                        GameOptionsOrigin::TacticalBattle => GameMode::TacticalCombat,
                    };
                }
            } else if game_mode == GameMode::LoadGame {
                save_load_panel_state.close();
                game_mode = GameMode::MainMenu;
            } else if matches!(game_mode, GameMode::Credits | GameMode::MultiplayerSetup) {
                game_mode = GameMode::MainMenu;
            } else if game_mode == GameMode::Galaxy {
                if targeting.is_some() {
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
                } else if troop_finder_state.is_open() || personnel_finder_state.is_open() {
                    // The Troop and Personnel Finders close on Escape in
                    // their own key slots, as the Fleet Finder.
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
                    show_save_load = false;
                    save_load_panel_state.close();
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
            && !show_save_load
            && !fleet_finder_state.is_open()
            && !troop_finder_state.is_open()
            && !personnel_finder_state.is_open()
            && !fleet_window_state.renaming()
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
                // Alt+G and Alt+U (commands 0xbc5/0xbc6 → menu items
                // 0x115/0x116, FUN_00422ce0) toggle Manage Garrisons and
                // Manage Production (FUN_00439d60).
                // Alt+A (0xbc9 → 0x11e) flips Agent Advice (FUN_00439e80).
                for (key, command) in [
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
                ] {
                    if is_key_pressed(key) {
                        apply_agent_command(&mut player_agent, command, "accelerator");
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
            if !alt_down
                && is_key_pressed(KeyCode::S)
                && !matches!(
                    game_mode,
                    GameMode::Cutscene { .. } | GameMode::VictoryModal { .. }
                )
            {
                if show_save_load {
                    save_load_panel_state.close();
                    show_save_load = false;
                } else {
                    save_slots = read_save_slots(&saves_dir);
                    save_load_panel_state.open_save();
                    show_save_load = true;
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
        let tick_events = if game_mode == GameMode::Galaxy {
            clock.advance(dt)
        } else {
            vec![]
        };

        if !tick_events.is_empty() {
            // Used by the Dabora 2 notification paths that need to timestamp
            // message-log entries. `current_tick` is re-bound further down for
            // the rest of the tick loop; this earlier binding is read-only.
            let economy_tick = tick_events.last().map_or(0, |e| e.tick);

            // Active movement orders are authoritative; repair stale orbit
            // indexes before economy and manufacturing inspect fleet presence.
            reconcile_fleet_orbits(&movement_state, &mut world);

            // The galaxy the day's loyalty shifts compare against.
            let loyalty_before =
                rebellion_core::loyalty::watch(&world, Some(victory_state.alliance_hq));

            // ── Economy (runs BEFORE manufacturing — affects production) ──────
            let economy_events = EconomySystem::advance_with_uprisings(
                &mut economy_state,
                &world,
                &tick_events,
                world.difficulty_index,
                &uprising_state,
            );
            for ev in &economy_events {
                match ev {
                    EconomyEvent::ControlResolved {
                        system,
                        new_control,
                    } => {
                        let previous = world.systems.get(*system).and_then(|s| s.control.faction());
                        if let Some(sys) = world.systems.get_mut(*system) {
                            sys.control = *new_control;
                        }
                        // FUN_00510d70 → FUN_004f6f40: the system's
                        // facilities follow its new holder.
                        world.hand_over_facilities(*system);
                        // Notification 0x100, System Joins / Declares
                        // Neutrality (FUN_00499760), while the support
                        // alone moved it. port: no regiment at the system
                        // stands for bit 0x2000 (FUN_00510d70).
                        if let Some(sys) = world.systems.get(*system) {
                            if sys.ground_units.is_empty() {
                                if let Some((title, text)) =
                                    rebellion_render::story_messages::control_change_texts(
                                        &sys.name,
                                        previous,
                                        new_control.faction(),
                                        player_faction == MissionFaction::Alliance,
                                        &message_texts,
                                    )
                                {
                                    msg_log.push(filed(
                                        GameMessage::at_system(
                                            tick_events.last().map_or(0, |event| event.tick),
                                            text,
                                            MessageCategory::Diplomacy,
                                            *system,
                                        )
                                        .with_title(title)
                                        .with_picture_if(Some(pictures::loyalty_change())),
                                        MessageRail::PopularSupport,
                                        Some(RailAudience::side(
                                            player_faction == MissionFaction::Alliance,
                                        )),
                                    ));
                                }
                            }
                        }
                    }
                    // Notification 0x145, Smuggling (FUN_00499130).
                    EconomyEvent::SmugglingPercentChanged {
                        system,
                        previous,
                        percent,
                    } => {
                        if let Some(sys) = world.systems.get(*system) {
                            if let Some(message) =
                                rebellion_render::story_messages::smuggling_message(
                                    &sys.name,
                                    sys.control.faction(),
                                    player_faction == MissionFaction::Alliance,
                                    *previous,
                                    *percent,
                                    tick_events.last().map_or(0, |event| event.tick),
                                    &message_texts,
                                )
                            {
                                msg_log.push(message);
                            }
                        }
                    }
                    // Notification 0x148, Near Uprising (FUN_00498cb0).
                    EconomyEvent::TroopSurplusChanged {
                        system,
                        previous,
                        surplus,
                    } => {
                        if let Some(sys) = world.systems.get(*system) {
                            let holder = sys.control.faction();
                            if let Some(message) =
                                rebellion_render::story_messages::near_uprising_message(
                                    &sys.name,
                                    holder,
                                    player_faction == MissionFaction::Alliance,
                                    // +0x88 bit 8, Garrisoned (FUN_0050ba90).
                                    !sys.ground_units.is_empty(),
                                    uprising_state.is_uprising(*system),
                                    *previous,
                                    *surplus,
                                    tick_events.last().map_or(0, |event| event.tick),
                                    &message_texts,
                                )
                            {
                                msg_log.push(
                                    message.with_picture_if(
                                        holder.and_then(pictures::near_uprising),
                                    ),
                                );
                            }
                        }
                    }
                    // Knesset Shamash-Bet Dabora 2 notification events —
                    // surface them in the interactive message log.
                    _ => {} // Telemetry-only events (collection rate, garrison, incidents, support change tier)
                }
            }

            // ── Manufacturing (blockaded systems are skipped) ─────────────────
            // `newly_idle` carries the K6 EVT_MANUFACTURING_IDLE transitions
            // for the interactive message log.
            // A manager with no yard left scraps its queue; yards in a
            // system in uprising or under blockade are suspended.
            scrap_yardless_queues(&world, &mut mfg_state);
            let suspended = suspended_systems(&blockade_state, &uprising_state);
            // Each yard cycle takes one refined material and ends with one
            // build point for its manager.
            let mfg_advance = advance_yards(
                &mut production_state,
                &world,
                &mut mfg_state,
                |system| suspended.contains(&system),
                &tick_events,
            );
            for completion in &mfg_advance.completions {
                // Apply the built item to the game world (ships, facilities, troops).
                rebellion_data::integrator::apply_build_completion_inner(completion, &mut world);
                let sys_name = world
                    .systems
                    .get(completion.system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                // Notification 0x107, Units Deployed (FUN_00497940).
                let (title, text) = rebellion_render::story_messages::deployed_texts(
                    &world,
                    completion.kind,
                    &sys_name,
                    &message_texts,
                )
                .map_or((None, format!("Construction complete at {sys_name}")), |(t, b)| {
                    (Some(t), b)
                });
                let mut message = GameMessage::at_system(
                    completion.tick,
                    text,
                    MessageCategory::Manufacturing,
                    completion.system,
                )
                .with_picture_if(pictures::deployed(&world, completion.kind));
                if let Some(title) = title {
                    message = message.with_title(title);
                }
                msg_log.push(filed(
                    message,
                    rebellion_render::story_messages::unit_message_rail(completion.kind, false),
                    system_audience(&world, completion.system),
                ));
                advisor_manufacturing_complete(&mut advisor_state, &sys_name);
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_sfx(SfxKind::BuildComplete, &audio_vol);
            }
            // Remote products complete at their facility, then travel to
            // their destination and complete there on arrival (F-030).
            let system_name = |world: &GameWorld, key| {
                world
                    .systems
                    .get(key)
                    .map_or_else(|| "unknown".to_string(), |s| s.name.clone())
            };
            for departure in &mfg_advance.departures {
                let origin = system_name(&world, departure.origin);
                let destination = system_name(&world, departure.destination);
                // Notification 0x107, Units Deployed (FUN_00497940): the
                // product is Deployed on completion (FUN_0052bee0), already
                // bound to its destination, which the message names.
                let (title, text) = rebellion_render::story_messages::deployed_texts(
                    &world,
                    departure.kind,
                    &destination,
                    &message_texts,
                )
                .map_or(
                    (None, format!("Construction complete at {origin}, en route to {destination}")),
                    |(title, body)| (Some(title), body),
                );
                let mut message = GameMessage::at_system(
                    departure.tick,
                    text,
                    MessageCategory::Manufacturing,
                    departure.destination,
                )
                .with_picture_if(pictures::deployed(&world, departure.kind));
                if let Some(title) = title {
                    message = message.with_title(title);
                }
                msg_log.push(filed(
                    message,
                    rebellion_render::story_messages::unit_message_rail(departure.kind, false),
                    system_audience(&world, departure.origin),
                ));
                advisor_manufacturing_complete(&mut advisor_state, &origin);
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_sfx(SfxKind::BuildComplete, &audio_vol);
            }
            delivery_state.depart(&world, &mfg_advance.departures);
            let delivered = delivery_state.advance(&world, &tick_events);
            for completion in &delivered.arrivals {
                rebellion_data::integrator::apply_build_completion_inner(completion, &mut world);
                // Notification 0x105 clearing, Unit Arrival (FUN_004981c0).
                let arrived_at = system_name(&world, completion.system);
                let (title, text) = rebellion_render::story_messages::arrival_texts(
                    &world,
                    completion.kind,
                    &arrived_at,
                    &message_texts,
                )
                .map_or((None, format!("Unit arrived at {arrived_at}")), |(t, b)| {
                    (Some(t), b)
                });
                let mut message = GameMessage::at_system(
                    completion.tick,
                    text,
                    MessageCategory::Manufacturing,
                    completion.system,
                )
                .with_picture_if(pictures::deployed(&world, completion.kind));
                if let Some(title) = title {
                    message = message.with_title(title);
                }
                msg_log.push(filed(
                    message,
                    rebellion_render::story_messages::unit_message_rail(completion.kind, true),
                    system_audience(&world, completion.system),
                ));
            }
            for lost in &delivered.lost {
                // Event 0x303, GameObjDestroyedOnArrivalNotif: Facility Lost
                // (FUN_00497690) speaks for a facility only, to its side.
                if let Some((title, text)) = rebellion_render::story_messages::facility_lost_texts(
                    &world,
                    lost.kind,
                    &system_name(&world, lost.destination),
                    &message_texts,
                ) {
                    msg_log.push(filed(
                        GameMessage::at_system(
                            lost.arrival_tick,
                            text,
                            MessageCategory::Manufacturing,
                            lost.origin,
                        )
                        .with_title(title),
                        MessageRail::Manufacturing,
                        Some(RailAudience::side(lost.side == Faction::Alliance)),
                    ));
                }
            }
            // Notification 0x160, Facility Idle (FUN_0048be60): a manager's
            // remaining count reached 0. Only the holder's side hears it,
            // and only while the system has a facility of that area.
            for &(system, area) in &mfg_advance.newly_idle {
                let Some(sys) = world.systems.get(system) else {
                    continue;
                };
                let player_holds = sys.control.faction()
                    == Some(if player_faction == MissionFaction::Alliance {
                        Faction::Alliance
                    } else {
                        Faction::Empire
                    });
                let picture = pictures::idle_yard(&world, system, area);
                if !player_holds || picture.is_none() {
                    continue;
                }
                if let Some((title, text)) = rebellion_render::story_messages::facility_idle_texts(
                    &sys.name,
                    area,
                    &message_texts,
                ) {
                    msg_log.push(filed(
                        GameMessage::at_system(
                            economy_tick,
                            text,
                            MessageCategory::Manufacturing,
                            system,
                        )
                        .with_title(title)
                        .with_picture_if(picture),
                        MessageRail::Manufacturing,
                        Some(RailAudience::side(player_faction == MissionFaction::Alliance)),
                    ));
                }
            }

            // ── Movement ────────────────────────────────────────────────────
            let arrivals = MovementSystem::advance(&mut movement_state, &tick_events);
            for arrival in &arrivals {
                let Some(applied) = apply_fleet_arrival(
                    &mut world,
                    &movement_state,
                    &mut troop_transport_state,
                    arrival,
                ) else {
                    continue;
                };
                let sys_name = world
                    .systems
                    .get(arrival.system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                // Notification 0xd, Unit Arrival.
                msg_log.push(filed(
                    GameMessage::at_system(
                        arrival.tick,
                        format!("Fleet arrived at {sys_name}"),
                        MessageCategory::Mission,
                        arrival.system,
                    )
                    .with_picture_if(pictures::arrival(
                        pictures::Arrival::Fleet,
                        pictures::reader(applied.is_alliance),
                    )),
                    MessageRail::Fleet,
                    Some(RailAudience::side(applied.is_alliance)),
                ));
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_sfx(SfxKind::FleetArrival, &audio_vol);
            }
            // Regiments travelling on their own (FUN_00556430, event 0x387).
            let now = tick_events.last().map_or(clock.tick, |event| event.tick);
            let regiments = troop_transport_state.advance_transit(&mut world, now);
            for transit in &regiments.arrived {
                let system = match transit.leg {
                    RegimentLeg::Surface(system) => Some(system),
                    RegimentLeg::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
                };
                let Some(system) = system else { continue };
                let name = world
                    .systems
                    .get(system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let audience = world
                    .troops
                    .get(transit.troop)
                    .map(|troop| RailAudience::side(troop.is_alliance));
                // Notification 0xd, Unit Arrival.
                msg_log.push(filed(
                    GameMessage::at_system(
                        transit.arrival_tick,
                        format!("Regiment arrived at {name}"),
                        MessageCategory::Mission,
                        system,
                    )
                    .with_picture_if(world.troops.get(transit.troop).and_then(
                        |troop| {
                            pictures::arrival(
                                pictures::Arrival::Units,
                                pictures::reader(troop.is_alliance),
                            )
                        },
                    )),
                    MessageRail::Fleet,
                    audience,
                ));
            }
            for transit in &regiments.lost {
                // Event 0x303, GameObjDestroyedOnArrivalNotif.
                msg_log.push(GameMessage::at_system(
                    transit.arrival_tick,
                    "Regiment lost on arrival: its destination no longer exists".to_string(),
                    MessageCategory::Mission,
                    transit.origin,
                ));
            }
            // Squadrons travelling on their own (squadrons::SquadronTransits).
            let (squadrons_arrived, squadrons_lost) =
                troop_transport_state.squadrons.advance(&mut world, now);
            for transit in &squadrons_arrived {
                let system = match transit.to {
                    SquadronPlace::Planet(system) => Some(system),
                    SquadronPlace::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
                };
                let Some(system) = system else { continue };
                let name = world
                    .systems
                    .get(system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let alliance = rebellion_core::squadrons::is_alliance(&world, transit.class);
                // Notification 0xd, Unit Arrival.
                msg_log.push(filed(
                    GameMessage::at_system(
                        transit.arrival_tick,
                        format!("Squadron arrived at {name}"),
                        MessageCategory::Mission,
                        system,
                    )
                    .with_picture_if(alliance.and_then(|alliance| {
                        pictures::arrival(pictures::Arrival::Units, pictures::reader(alliance))
                    })),
                    MessageRail::Fleet,
                    alliance.map(RailAudience::side),
                ));
            }
            for transit in &squadrons_lost {
                // Event 0x303, GameObjDestroyedOnArrivalNotif.
                msg_log.push(GameMessage::at_system(
                    transit.arrival_tick,
                    "Squadron lost on arrival: its destination no longer exists".to_string(),
                    MessageCategory::Mission,
                    transit.origin,
                ));
            }

            let current_tick = tick_events.last().map_or(0, |event| event.tick);

            // ── Combat ──────────────────────────────────────────────────────
            // After fleet arrivals, check every system for opposing fleets.
            // Collect combat triggers first (immutable world borrow).
            let combat_triggers: Vec<_> = world
                .systems
                .keys()
                .filter_map(|sys_key| {
                    // Combat cooldown: skip systems that had combat within last 5 ticks.
                    if let Some(&last_battle) = combat_cooldowns.get(&sys_key) {
                        if current_tick < last_battle.saturating_add(5) {
                            return None;
                        }
                    }
                    // FUN_00509710 / FUN_00516d50: both sides present, by a
                    // fleet or a squadron on the planet, start a battle.
                    let (alliance, empire) =
                        rebellion_core::squadrons::sides_present(&world, sys_key);
                    (alliance && empire).then_some(sys_key)
                })
                .collect();

            for sys_key in combat_triggers {
                // The planet's squadrons go up for the battle and land after
                // it (squadrons::Sortie).
                let sorties = rebellion_core::squadrons::scramble(&mut world, sys_key);
                let first_fleet = |alliance: bool| {
                    world.systems[sys_key].fleets.iter().copied().find(|&key| {
                        world
                            .fleets
                            .get(key)
                            .is_some_and(|fleet| fleet.is_alliance == alliance)
                    })
                };
                let (Some(atk_fleet), Some(def_fleet)) = (first_fleet(true), first_fleet(false))
                else {
                    rebellion_core::squadrons::recover(&mut world, &sorties);
                    continue;
                };
                let sys_name = world
                    .systems
                    .get(sys_key)
                    .map_or_else(|| "Unknown".into(), |s| s.name.clone());

                // Check if the player is involved in this battle.
                let player_is_alliance = player_faction == MissionFaction::Alliance;
                let atk_is_alliance = world.fleets.get(atk_fleet).is_some_and(|f| f.is_alliance);
                let def_is_alliance = world.fleets.get(def_fleet).is_some_and(|f| f.is_alliance);
                // Player is involved if either fleet belongs to the player's faction.
                let player_involved = (player_is_alliance == atk_is_alliance)
                    || (player_is_alliance == def_is_alliance);

                if player_involved && game_mode == GameMode::Galaxy {
                    // Transition to tactical combat view for player-involved battles.
                    let player_is_attacker = if player_is_alliance {
                        atk_is_alliance
                    } else {
                        !atk_is_alliance
                    };
                    if let Err(error) = tactical_flow::begin_player_battle(
                        &world,
                        tactical_flow::BattleEntry {
                            system: sys_key,
                            attacker: atk_fleet,
                            defender: def_fleet,
                            player_is_attacker,
                            tick: current_tick,
                            rng_seed: sim_rng.gen::<u32>(),
                        },
                        &mut tactical_state,
                        &mut combat_cooldowns,
                        &mut msg_log,
                        &mut game_mode,
                    ) {
                        eprintln!("Skipping invalid tactical battle at {sys_name}: {error:?}");
                        rebellion_core::squadrons::recover(&mut world, &sorties);
                        continue;
                    }
                    battle_sorties = sorties;
                    tactical_state.set_display_options(game_options_state.tactical_flags());
                    #[cfg(not(target_arch = "wasm32"))]
                    audio_engine.play_sfx(SfxKind::CombatStart, &audio_vol);
                    break; // Handle one player battle at a time.
                }

                // AI vs AI: auto-resolve as before.
                let battle_before = rebellion_core::loyalty::battle_watch(&world, sys_key);
                let attacker_is_alliance = world.fleets.get(atk_fleet).map(|f| f.is_alliance);
                let combat_rolls: Vec<f64> = (0..256).map(|_| sim_rng.gen::<f64>()).collect();
                // FUN_004362b0: each AI side's retreat order (FUN_00485d50).
                let ais: Vec<&AIState> =
                    std::iter::once(&ai_state).chain(&secondary_ai_state).collect();
                let retreat_allowed = rebellion_core::ai_strategy::retreat_orders(
                    &ais,
                    &world,
                    &movement_state,
                    &troop_transport_state,
                    sys_key,
                );
                // FUN_0040a050 state 8: an abstract battle (FUN_005ae9c0).
                let (space_result, _) = rebellion_core::abstract_battle::resolve_space_battle(
                    &world,
                    sys_key,
                    &[atk_fleet],
                    &[def_fleet],
                    rebellion_core::abstract_battle::abstract_seed(&combat_rolls),
                    current_tick,
                    retreat_allowed,
                );

                // Apply ship damage: reduce counts proportional to destroyed hulls.
                apply_space_combat_result(&space_result, &mut world);
                rebellion_core::squadrons::recover(&mut world, &sorties);
                // FUN_00517270 -> FUN_00560d00: the winner's loyalty shift.
                let winner = battle_winner(space_result.winner, attacker_is_alliance);
                rebellion_core::loyalty::settle_battle(&mut world, battle_before, sys_key, winner);
                // FUN_00516ef0: the loser's fleets and planet squadrons leave
                // on Autorouting.
                rebellion_core::abstract_battle::withdraw_loser(
                    &mut world,
                    &mut movement_state,
                    &mut troop_transport_state.squadrons,
                    sys_key,
                    winner,
                    current_tick,
                );
                troop_transport_state.destroy_untransportable_cargo(&mut world);
                // Record combat cooldown to prevent infinite re-trigger on draws.
                combat_cooldowns.insert(sys_key, current_tick);

                let winner_str = match space_result.winner {
                    CombatSide::Attacker => "Alliance victory",
                    CombatSide::Defender => "Empire victory",
                    CombatSide::Draw => "Draw",
                };
                msg_log.push(GameMessage::at_system(
                    current_tick,
                    format!("Space battle at {sys_name} — {winner_str}"),
                    MessageCategory::Combat,
                    sys_key,
                ));
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_sfx(SfxKind::CombatStart, &audio_vol);
            }

            // One side's regiments alone take a planet (the simulation's
            // step 3b); carried regiments stay aboard until ordered, and no
            // ground battle happens outside a Planetary Assault
            // (ghidra/notes/planetary-assault.md).
            tactical_flow::occupy(&mut world, current_tick, &mut msg_log);

            // Observe embarked regiments after arrivals and landings, so a
            // fleet that arrives and is ordered away again this tick still
            // copies the arrival system's withdraw percent.
            let mut running_regiments = if tick_events.is_empty() {
                Vec::new()
            } else {
                BlockadeSystem::running_regiments(
                    &mut blockade_state,
                    &world,
                    &movement_state,
                    &troop_transport_state,
                )
            };

            // ── Fog of war ──────────────────────────────────────────────────
            let alliance_reveals =
                FogSystem::advance(&mut fog_alliance_state, &world, &movement_state);
            let empire_reveals = FogSystem::advance(&mut fog_empire_state, &world, &movement_state);
            let reveals = if player_faction == MissionFaction::Alliance {
                alliance_reveals
            } else {
                empire_reveals
            };
            for reveal in &reveals {
                let sys_name = world
                    .systems
                    .get(reveal.system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                msg_log.push(GameMessage::at_system(
                    tick_events.last().unwrap().tick,
                    format!("System {sys_name} revealed"),
                    MessageCategory::Event,
                    reveal.system,
                ));
            }

            // ── Missions ────────────────────────────────────────────────────
            let mission_rolls: Vec<f64> = (0..mission_state.len()
                * tick_events.len()
                * rebellion_core::missions::ROLLS_PER_MISSION)
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            let mission_advance = MissionSystem::advance(
                &mut mission_state,
                &world,
                &uprising_state,
                &research_state,
                &tick_events,
                &mission_rolls,
            );
            // Member moves and releases come before the results.
            rebellion_core::missions::apply_advance_effects(&mut world, &mission_advance.effects);
            push_advance_injuries(&mut msg_log, &world, &mission_advance.effects, current_tick);
            // Notification 0x361 (FUN_0048c9c0): a member learned the traitor.
            for effect in &mission_advance.effects {
                if let MissionEffect::TraitorIdentified { character, traitor } = effect {
                    if let (Some(discoverer), Some(traitor)) =
                        (world.characters.get(*character), world.characters.get(*traitor))
                    {
                        if let Some(message) =
                            rebellion_render::story_messages::discovers_traitor_message(
                                discoverer,
                                traitor,
                                current_tick,
                                &message_texts,
                            )
                        {
                            msg_log.push(message);
                        }
                    }
                }
            }
            // The encounter class (vtable 0x65b408): Force detection and the
            // confrontation's 0x360 and Luke's 0x211.
            for message in rebellion_render::story_messages::EncounterNotes::gather(
                &world,
                &mission_advance.effects,
            )
            .messages(&world, &mission_state, current_tick, &message_texts)
            {
                msg_log.push(message);
            }
            // ── Dagobah: timer 0x392 and the check FUN_00544030 ─────────────
            let dagobah_rolls: Vec<f64> =
                (0..rebellion_core::dagobah::DagobahSystem::roll_budget())
                    .map(|_| sim_rng.gen::<f64>())
                    .collect();
            if let Some(pulse) = rebellion_core::dagobah::DagobahSystem::advance(
                &mut dagobah_state,
                &world,
                &mission_state,
                &tick_events,
                &dagobah_rolls,
            ) {
                if rebellion_core::dagobah::dispatch(&mut mission_state, &mut world, &pulse)
                    .is_some()
                {
                    if let Some(message) = rebellion_render::story_messages::luke_goes_to_dagobah(
                        pulse.tick,
                        &message_texts,
                    ) {
                        msg_log.push(message);
                    }
                }
            }
            // Notification 0x210 when the Dagobah mission sets
            // DagobahCompleted (FUN_005751d0).
            for effect in &mission_advance.effects {
                if let MissionEffect::DagobahCompleted { character, .. } = effect {
                    if let Some(message) = rebellion_render::story_messages::luke_leaves_dagobah(
                        &world,
                        *character,
                        current_tick,
                        &message_texts,
                    ) {
                        msg_log.push(message);
                    }
                }
            }
            // ── The Final Battle: FUN_00544da0 .. FUN_00544fc0 ──────────────
            let final_battle_rolls: Vec<f64> =
                (0..rebellion_core::final_battle::FinalBattleSystem::roll_budget())
                    .map(|_| sim_rng.gen::<f64>())
                    .collect();
            for step in rebellion_core::final_battle::FinalBattleSystem::advance(
                &mut final_battle_state,
                &world,
                &mission_state,
                &tick_events,
                &final_battle_rolls,
            ) {
                rebellion_core::final_battle::perform(&step, &mut mission_state, &mut world);
                // FUN_0048e3c0 on 0x220.
                if let rebellion_core::final_battle::FinalBattleStep::Battle(battle) = step {
                    if let Some(message) = rebellion_render::story_messages::final_battle_message(
                        &world,
                        battle.luke,
                        battle.tick,
                        &message_texts,
                    ) {
                        msg_log.push(message);
                    }
                }
            }
            // ── The bounty hunters: timer 0x393 (FUN_00546fa0) ─────────────
            let jabba_rolls: Vec<f64> = (0..rebellion_core::jabba::JabbaSystem::roll_budget())
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            for step in rebellion_core::jabba::JabbaSystem::advance(
                &mut jabba_state,
                &world,
                &mission_state,
                &tick_events,
                &jabba_rolls,
            ) {
                let dispatched =
                    rebellion_core::jabba::perform(&step, &mut mission_state, &mut world);
                // Notification 0x200 (FUN_0048c4c0), Rescue Attempt.
                // port: posted as the Palace mission is dispatched; the
                // original posts it as the rescuer sets out (EnrouteActive).
                if let (rebellion_core::jabba::JabbaStep::Rescue { rescuer, .. }, Some(_)) =
                    (step, dispatched)
                {
                    if let Some(message) = world.characters.get(rescuer).and_then(|c| {
                        rebellion_render::story_messages::rescue_attempt_message(
                            c,
                            current_tick,
                            &message_texts,
                        )
                    }) {
                        msg_log.push(message);
                    }
                }
            }
            // ── SeatOfPower (FUN_005617b0): notification 0x230, Emperor
            // Arrives at Coruscant (FUN_0048ce80) when set ──────────────
            if rebellion_core::seat_of_power::SeatOfPowerSystem::advance(
                &mut seat_of_power_state,
                &world,
            ) == Some(true)
            {
                if let Some(message) = rebellion_render::story_messages::emperor_arrives_message(
                    current_tick,
                    &message_texts,
                ) {
                    msg_log.push(message);
                }
            }
            ai_state.free_mission_members(&mission_advance.effects);
            if let Some(second_ai) = secondary_ai_state.as_mut() {
                second_ai.free_mission_members(&mission_advance.effects);
            }
            ai_state.note_mission_results(&mission_advance.results);
            if let Some(second_ai) = secondary_ai_state.as_mut() {
                second_ai.note_mission_results(&mission_advance.results);
            }

            for result in &mission_advance.results {
                apply_mission_result(
                    result,
                    &mut world,
                    &mut research_state,
                    &mut msg_log,
                    &message_texts,
                    #[cfg(not(target_arch = "wasm32"))]
                    &mut audio_engine,
                    #[cfg(not(target_arch = "wasm32"))]
                    &audio_vol,
                );
                // Notification 0x212 (FUN_0048fa00), after Han's capture;
                // notification 0x1e0 case 5 (FUN_00490340) for a rescuer
                // the Palace mission's failure leaves with Jabba.
                for effect in &result.effects {
                    if let MissionEffect::BountyAttack { character, .. } = effect {
                        for message in rebellion_render::story_messages::bounty_messages(
                            &world,
                            *character,
                            current_tick,
                            &message_texts,
                        ) {
                            msg_log.push(message);
                        }
                    }
                    // FUN_004fc080: an object destroyed by an assassination
                    // (reason 7) posts 0x306, Killed by Imperial Assassins
                    // (FUN_0048b050); one destroyed by sabotage (reason 6)
                    // posts 0x305, Death Star Sabotaged for the Death Star
                    // (FUN_0048d470).
                    let system_name = world
                        .systems
                        .get(result.target_system)
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    if let MissionEffect::CharacterKilled { character, .. } = effect {
                        if result.kind == MissionKind::Assassination {
                            if let Some(message) = world.characters.get(*character).and_then(|c| {
                                rebellion_render::story_messages::assassins_message(
                                    c,
                                    &system_name,
                                    current_tick,
                                    &message_texts,
                                )
                            }) {
                                msg_log.push(message);
                            }
                        }
                    }
                    if let MissionEffect::TargetSabotaged {
                        target: rebellion_core::missions::MissionTarget::DeathStar(_),
                        system,
                    } = effect
                    {
                        let name = world
                            .systems
                            .get(*system)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();
                        if let Some(message) =
                            rebellion_render::story_messages::death_star_sabotaged_message(
                                &name,
                                current_tick,
                                &message_texts,
                            )
                        {
                            msg_log.push(message);
                        }
                    }
                    // Notification 0x231 (FUN_0048b6e0) as the Pickup sets
                    // PickupInProgress.
                    if matches!(effect, MissionEffect::PickupInProgress) {
                        if let Some(message) =
                            rebellion_render::story_messages::jabba_delivers_message(
                                current_tick,
                                &message_texts,
                            )
                        {
                            msg_log.push(message);
                        }
                    }
                    if let MissionEffect::CharacterCaptured { character, .. } = effect {
                        if rebellion_core::jabba::held_at_palace(&world, *character) {
                            if let Some(message) =
                                rebellion_render::story_messages::jabba_captures_message(
                                    &world.characters[*character],
                                    current_tick,
                                    &message_texts,
                                )
                            {
                                msg_log.push(message);
                            }
                        }
                    }
                }
                for ended in rebellion_data::integrator::apply_mission_state_effects(
                    &result.effects,
                    &world,
                    result.tick,
                    &mut uprising_state,
                ) {
                    if let rebellion_core::uprising::UprisingEvent::UprisingEnded { system, tick } =
                        ended
                    {
                        let name = world
                            .systems
                            .get(system)
                            .map_or_else(|| "unknown".into(), |s| s.name.clone());
                        macroquad::logging::info!(
                            "[interface] uprising_subdued system={} tick={}",
                            name,
                            tick
                        );
                        // Notification 0x14b, Uprising Ends (FUN_00499460),
                        // for both sides of the held system.
                        let holder = world.systems.get(system).and_then(|s| s.control.faction());
                        let (title, text, audience) =
                            match rebellion_render::story_messages::uprising_texts(
                                &name,
                                holder,
                                false,
                                &message_texts,
                            ) {
                                Some((title, body)) => {
                                    (Some(title), body, Some(RailAudience::Both))
                                }
                                None => (
                                    None,
                                    format!("Uprising subdued at {name}"),
                                    system_audience(&world, system),
                                ),
                            };
                        let mut message = GameMessage::at_system(
                            tick,
                            text,
                            MessageCategory::Diplomacy,
                            system,
                        )
                        .with_picture_if(pictures::uprising_ends(pictures::system_side(
                            &world, system,
                        )));
                        if let Some(title) = title {
                            message = message.with_title(title);
                        }
                        msg_log.push(filed(message, MessageRail::PopularSupport, audience));
                    }
                }

                // Advisor trigger for player faction missions.
                if result.faction == player_faction {
                    let kind_name = format!("{:?}", result.kind);
                    let success =
                        result.outcome == rebellion_core::missions::MissionOutcome::Success;
                    advisor_mission_result(&mut advisor_state, &kind_name, success);
                }
            }

            // ── Character escapes ────────────────────────────────────────────
            // Each captive draws at most an attempt and its next delay.
            let escape_rolls: Vec<f64> = (0..2 * world.characters.len())
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            let escape_effects = MissionSystem::advance_escapes(
                &mut mission_state,
                &world,
                current_tick,
                &escape_rolls,
            );
            for effect in &escape_effects {
                if let MissionEffect::MemberMoved { member, to } = effect {
                    rebellion_core::missions::move_member(&mut world, *member, *to);
                } else if let MissionEffect::MemberEnteredAbode { member, abode } = effect {
                    rebellion_core::missions::enter_abode(&mut world, *member, *abode);
                } else if let MissionEffect::CharacterEscaped {
                    character,
                    escaped_to_alliance,
                } = effect
                {
                    if let Some(c) = world.characters.get_mut(*character) {
                        c.is_alliance = *escaped_to_alliance;
                        c.is_empire = !*escaped_to_alliance;
                        c.is_captive = false;
                        c.captured_by = None;
                        c.capture_tick = None;
                    }
                    for (_, fleet) in &mut world.fleets {
                        fleet.characters.retain(|&k| k != *character);
                    }
                    let name = world
                        .characters
                        .get(*character)
                        .map_or_else(|| "Unknown".into(), |c| c.name.clone());
                    msg_log.push(
                        GameMessage::new(
                            current_tick,
                            format!("{name} has escaped captivity!"),
                            MessageCategory::Event,
                        )
                        .with_picture(pictures::escaped()),
                    );
                }
            }

            // ── Events ──────────────────────────────────────────────────────
            let event_rolls: Vec<f32> = (0..16).map(|_| sim_rng.gen::<f32>()).collect();
            let fired_events =
                EventSystem::advance(&mut event_state, &world, &tick_events, &event_rolls);

            // #F7 + #A3: call the pub'd integrator helper. DisplayMessage
            // routes through `GameEffect::StoryMessageDisplayed` and
            // `SpawnSpecialForce` resolves via `current_system` + fallback
            // to `MovementState::orders()`. The effect buffer drains into
            // `msg_log` immediately below.
            for fired in &fired_events {
                let mut story_effects_out: Vec<rebellion_core::effects::GameEffect> = Vec::new();
                rebellion_data::integrator::apply_event_action_to_world(
                    &fired.actions,
                    &mut world,
                    &mut story_effects_out,
                    fired.tick,
                    &movement_state,
                );
                // Drain StoryMessageDisplayed + SpecialForceSpawned effects into
                // the interactive message log. SpecialForceUnit arena wiring is
                // handled in apply_event_action_to_world; here we just log.
                for eff in story_effects_out {
                    use rebellion_core::effects::GameEffect;
                    match eff {
                        GameEffect::StoryMessageDisplayed { text, .. } => {
                            // A story event is a message the player reads in
                            // the Message window (DEV-UI-019 retired).
                            msg_log.push(rebellion_render::story_messages::story_message(
                                &world,
                                fired.event_id,
                                &fired.actions,
                                text,
                                current_tick,
                                cockpit_state.faction,
                                &message_texts,
                                MessageCategory::Event,
                            ));
                        }
                        GameEffect::SpecialForceSpawned {
                            at_system,
                            is_alliance,
                        } => {
                            let name = world
                                .systems
                                .get(at_system)
                                .map_or_else(|| "unknown".into(), |s| s.name.clone());
                            let side = if is_alliance { "Alliance" } else { "Imperial" };
                            msg_log.push(GameMessage::at_system(
                                current_tick,
                                format!("{side} special force lands at {name}"),
                                MessageCategory::Event,
                                at_system,
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // ── Story cutscene triggers (C1–C8) ─────────────────────────────
            // When a story event fires that maps to a cutscene file (101–108),
            // launch the cutscene. Only the first matching event triggers a
            // cutscene per tick. Story cutscenes return to Galaxy when done.
            for fired in &fired_events {
                if let Some(number) = story_event_to_cutscene(fired.event_id) {
                    let path_str = story_cutscene_path(number);
                    cutscene_player = open_cutscene(
                        Path::new(&path_str),
                        audio_vol.cutscene_volume(),
                        &mut msg_log,
                        current_tick,
                        #[cfg(not(target_arch = "wasm32"))]
                        &mut audio_engine,
                    );
                    if cutscene_player.is_some() {
                        game_mode = GameMode::Cutscene {
                            kind: CutsceneKind::Story(number),
                        };
                        break;
                    }
                }
            }

            // ── AI: each day, as many passes as the speed has sub-ticks, one
            // side then the other (FUN_0041cf20 -> FUN_004361a0); each
            // pass's orders apply before the next, and a new day clears each
            // side's order count (FUN_00485de0).
            let ai_passes = rebellion_core::ai::passes_per_day(clock.speed);
            let mut bombard =
                BombardEnv::new(sim_rng.gen::<u64>(), Some(victory_state.alliance_hq));
            for event in &tick_events {
                ai_state.begin_day();
                ai_state.set_game_type(campaign_config.victory_conditions);
                if let Some(ref mut second_ai) = secondary_ai_state {
                    second_ai.begin_day();
                    second_ai.set_game_type(campaign_config.victory_conditions);
                }
                // The AI follows its ships by id (GameWorld::number_ships).
                world.number_ships();
                let ai_started = macroquad::time::get_time();
                for _ in 0..ai_passes {
                    let ai_actions = AISystem::pass(
                        &mut ai_state,
                        &world,
                        &mfg_state,
                        &mission_state,
                        &movement_state,
                        &economy_state,
                        &troop_transport_state,
                        event.tick,
                        &game_config,
                        &research_state,
                        &uprising_state,
                    );
                    apply_ai_actions(
                        &ai_actions,
                        &mut ai_state,
                        &mut mission_state,
                        &mut mfg_state,
                        &mut movement_state,
                        &mut troop_transport_state,
                        &mut production_state,
                        &mut world,
                        &mut msg_log,
                        event.tick,
                        &mut bombard,
                        #[cfg(not(target_arch = "wasm32"))]
                        &mut audio_engine,
                        #[cfg(not(target_arch = "wasm32"))]
                        &audio_vol,
                    );
                    // ── Dual AI (second faction)
                    if let Some(ref mut second_ai) = secondary_ai_state {
                        let second_actions = AISystem::pass(
                            second_ai,
                            &world,
                            &mfg_state,
                            &mission_state,
                            &movement_state,
                            &economy_state,
                            &troop_transport_state,
                            event.tick,
                            &game_config,
                            &research_state,
                            &uprising_state,
                        );
                        apply_ai_actions(
                            &second_actions,
                            second_ai,
                            &mut mission_state,
                            &mut mfg_state,
                            &mut movement_state,
                            &mut troop_transport_state,
                            &mut production_state,
                            &mut world,
                            &mut msg_log,
                            event.tick,
                            &mut bombard,
                            #[cfg(not(target_arch = "wasm32"))]
                            &mut audio_engine,
                            #[cfg(not(target_arch = "wasm32"))]
                            &audio_vol,
                        );
                    }
                }
                macroquad::logging::info!(
                    "[ai] day={} speed={:?} passes={} orders={} second_orders={} ms={:.1}",
                    event.tick,
                    clock.speed,
                    ai_passes,
                    ai_state.orders_today,
                    secondary_ai_state.as_ref().map_or(0, |ai| ai.orders_today),
                    (macroquad::time::get_time() - ai_started) * 1000.0
                );
            }

            // System_vtd0: the day's bombardment and assault losses count
            // toward each blockade's (DestroyedReason 0xf, 0xe).
            for (system, losses) in &bombard.losses {
                blockade_state.record_losses(*system, *losses);
            }
            present_assault_reports(
                std::mem::take(&mut bombard.reports),
                &world,
                player_faction,
                &message_texts,
                &mut assault_summary_state,
                &mut msg_log,
                current_tick,
            );

            // A Destroy System order's Death Star destroyed these (FUN_0058d990).
            for system in bombard.destroyed_systems() {
                destroy_planet(
                    &mut world,
                    system,
                    &bombard.killed_at(system),
                    &mut mfg_state,
                    &mut blockade_state,
                    &mut msg_log,
                    current_tick,
                    &message_texts,
                );
            }

            // ── The player's agent (FUN_00439a10): once a day, its running
            // modules issue Destination and build orders. Not in dual-AI
            // mode, where an AI plays the player's side.
            if secondary_ai_state.is_none() {
                for _ in &tick_events {
                    let orders = player_agent.advance(
                        &world,
                        &mfg_state,
                        &economy_state,
                        player_faction == MissionFaction::Alliance,
                    );
                    for order in &orders {
                        macroquad::logging::info!("[agent] order={:?}", order);
                    }
                    rebellion_core::agent_automation::apply_orders(&orders, &mut mfg_state);
                }
            }

            // ── Blockade ─────────────────────────────────────────────────────
            let mut blockade_events =
                BlockadeSystem::advance(&mut blockade_state, &world, &tick_events);
            // Event305_SystemBlockadeChanged_Handle -> FUN_00561280: each
            // ended blockade's loyalty shift for its assault and bombardment
            // losses.
            for (system, blockade) in blockade_state.take_ended() {
                rebellion_core::loyalty::settle_blockade(&mut world, system, blockade);
            }
            if let Some(last) = tick_events.last() {
                running_regiments.extend(BlockadeSystem::running_regiments(
                    &mut blockade_state,
                    &world,
                    &movement_state,
                    &troop_transport_state,
                ));
                let running_rolls: Vec<f64> = (0..running_regiments.len())
                    .map(|_| sim_rng.gen::<f64>())
                    .collect();
                blockade_events.extend(BlockadeSystem::resolve_running(
                    &running_regiments,
                    &running_rolls,
                    last.tick,
                ));
            }
            for evt in &blockade_events {
                match evt {
                    rebellion_core::blockade::BlockadeEvent::BlockadeStarted { system, tick } => {
                        let name = world
                            .systems
                            .get(*system)
                            .map_or_else(|| "unknown".into(), |s| s.name.clone());
                        // Notification 0x14e, Blockade (FUN_004960f0): the
                        // holder and the blockader each read their own text;
                        // it needs a fleet at the system (FUN_004ffdd0).
                        let holder = world.systems.get(*system).and_then(|s| s.control.faction());
                        let fleet = world
                            .systems
                            .get(*system)
                            .and_then(|s| s.fleets.first())
                            .and_then(|&fleet| world.fleet_name(fleet))
                            .map(str::to_owned);
                        let original = fleet.as_deref().and_then(|fleet| {
                            rebellion_render::story_messages::blockade_texts(
                                &name,
                                holder,
                                fleet,
                                player_faction == MissionFaction::Alliance,
                                &message_texts,
                            )
                        });
                        let (title, text, audience) = match original {
                            Some((title, body)) => (
                                Some(title),
                                body,
                                Some(RailAudience::side(
                                    player_faction == MissionFaction::Alliance,
                                )),
                            ),
                            None if message_texts.is_empty() => (
                                None,
                                format!("Blockade established at {name}"),
                                Some(RailAudience::Both),
                            ),
                            None => continue,
                        };
                        let mut message = GameMessage::at_system(
                            *tick,
                            text,
                            MessageCategory::Combat,
                            *system,
                        )
                        .with_picture_if(pictures::blockade(pictures::system_side(
                            &world, *system,
                        )));
                        if let Some(title) = title {
                            message = message.with_title(title);
                        }
                        msg_log.push(filed(message, MessageRail::Conflict, audience));
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
                    rebellion_core::blockade::BlockadeEvent::SupportShifted {
                        system,
                        side,
                        support_delta,
                        ..
                    } => {
                        // Timer 0x386 only sets the support (FUN_0050c9f0).
                        rebellion_core::uprising::shift_support(
                            &mut world,
                            *system,
                            *side,
                            *support_delta,
                        );
                    }
                    rebellion_core::blockade::BlockadeEvent::TroopDestroyed {
                        system,
                        troop,
                        tick,
                    } => {
                        // Event 0x340, Evacuation Losses (FUN_00495e90), to
                        // the regiment's side; named before it goes.
                        let regiment = world.troops.get(*troop).map(|unit| {
                            let (name, _) = rebellion_render::manufacturing_window::product(
                                &world,
                                rebellion_core::manufacturing::BuildableKind::Troop(
                                    unit.class_dat_id,
                                ),
                            );
                            (name, unit.is_alliance)
                        });
                        // The regiment was aboard a fleet leaving the system.
                        troop_transport_state.destroy_embarked(&mut world, *troop);
                        let name = world
                            .systems
                            .get(*system)
                            .map_or_else(|| "unknown".into(), |s| s.name.clone());
                        let original = regiment.as_ref().and_then(|(regiment, _)| {
                            rebellion_render::story_messages::evacuation_losses_texts(
                                regiment,
                                &name,
                                &message_texts,
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
                                .with_picture_if(Some(pictures::evacuation_losses(
                                    cockpit_state.faction,
                                )));
                        if let Some(title) = title {
                            message = message.with_title(title);
                        }
                        msg_log.push(filed(message, MessageRail::Conflict, Some(audience)));
                    }
                }
            }

            // ── Mines, refineries and the side stockpiles ───────────────────
            let production_rolls: Vec<f64> = (0..world.production_facilities.len())
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            ProductionSystem::advance(
                &mut production_state,
                &world,
                &mfg_state,
                |system| blockade_state.is_blockaded(system) || uprising_state.is_uprising(system),
                |system| rebellion_core::economy::production_modifier_at(&economy_state, system),
                &tick_events,
                &production_rolls,
            );

            // ── The yards' own research: each side's timer 0x381 ────────────
            let tally_rolls = [sim_rng.gen::<f64>(), sim_rng.gen::<f64>()];
            // Each tree's Done flag before the tally (FUN_0052f440 posts
            // 0x129..0x12b when it turns on).
            let done_before: Vec<(bool, rebellion_core::research::TechType, bool)> = [true, false]
                .into_iter()
                .flat_map(|alliance| {
                    [
                        rebellion_core::research::TechType::Ship,
                        rebellion_core::research::TechType::Troop,
                        rebellion_core::research::TechType::Facility,
                    ]
                    .into_iter()
                    .map(move |tech| (alliance, tech))
                })
                .map(|(alliance, tech)| (alliance, tech, research_state.progress(alliance, tech).done))
                .collect();
            let tallies = rebellion_core::research::ResearchSystem::advance_tally(
                &mut research_state,
                &world,
                |system| blockade_state.is_blockaded(system) || uprising_state.is_uprising(system),
                &tick_events,
                &tally_rolls,
            );
            for tally in &tallies {
                let faction_name = if tally.faction_is_alliance {
                    "Alliance"
                } else {
                    "Empire"
                };
                let tree_name = match tally.tech {
                    rebellion_core::research::TechType::Ship => "Ship",
                    rebellion_core::research::TechType::Troop => "Troop",
                    rebellion_core::research::TechType::Facility => "Facility",
                };
                let tick = tick_events.last().map_or(0, |event| event.tick);
                for level in &tally.reached {
                    // Notifications 0x125..0x127, Research Complete
                    // (FUN_00499aa0): the class the level opens.
                    let (title, text) = match rebellion_render::story_messages::research_complete_texts(
                        &world,
                        tally.faction_is_alliance,
                        tally.tech,
                        *level,
                        &message_texts,
                    ) {
                        Some((title, body)) => (Some(title), body),
                        None if message_texts.is_empty() => (
                            None,
                            format!("{faction_name} {tree_name} tech advanced to level {level}"),
                        ),
                        None => continue,
                    };
                    let mut message = GameMessage::new(tick, text, MessageCategory::Event)
                        .with_picture_if(Some(pictures::research()));
                    if let Some(title) = title {
                        message = message.with_title(title);
                    }
                    msg_log.push(filed(
                        message,
                        MessageRail::Manufacturing,
                        Some(RailAudience::side(tally.faction_is_alliance)),
                    ));
                }
            }
            // Notifications 0x129..0x12b, Research Exhausted (FUN_0048abc0).
            for (alliance, tech, was_done) in done_before {
                if was_done || !research_state.progress(alliance, tech).done {
                    continue;
                }
                if let Some((title, text)) =
                    rebellion_render::story_messages::research_exhausted_texts(tech, &message_texts)
                {
                    msg_log.push(filed(
                        GameMessage::new(
                            tick_events.last().map_or(0, |event| event.tick),
                            text,
                            MessageCategory::Event,
                        )
                        .with_title(title)
                        .with_picture_if(Some(pictures::research())),
                        MessageRail::Manufacturing,
                        Some(RailAudience::side(alliance)),
                    ));
                }
            }

            // ── Maintenance shortfall: each side's autoscrap timer 0x382 ────
            let autoscrap_rolls = [sim_rng.gen::<f64>(), sim_rng.gen::<f64>()];
            let autoscraps = rebellion_core::autoscrap::AutoscrapSystem::advance(
                &mut autoscrap_state,
                &world,
                &troop_transport_state,
                &mfg_state,
                |system| blockade_state.is_blockaded(system) || uprising_state.is_uprising(system),
                &tick_events,
                &autoscrap_rolls,
            );
            for scrap in &autoscraps {
                rebellion_core::autoscrap::apply(
                    &mut world,
                    &mut production_state,
                    &mut troop_transport_state,
                    scrap,
                );
                let alliance = scrap.side == Faction::Alliance;
                let name = scrap
                    .system
                    .and_then(|system| world.systems.get(system))
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                // Notification 0x304 (GameObjDestroyedAutoscrap), Maintenance
                // Shortfall (FUN_00498970): TEXTSTRA 0x7080-0x7082.
                let system_name = scrap.system.map(|_| name.as_str());
                let Some((title, text)) = rebellion_render::story_messages::maintenance_texts(
                    system_name,
                    &scrap.unit,
                    &message_texts,
                ) else {
                    continue;
                };
                let message = match scrap.system {
                    Some(system) => {
                        GameMessage::at_system(scrap.tick, text, MessageCategory::Event, system)
                    }
                    None => GameMessage::new(scrap.tick, text, MessageCategory::Event),
                };
                msg_log.push(filed(
                    message.with_title(title).with_picture_if(Some(pictures::units_destroyed(
                        pictures::reader(alliance),
                    ))),
                    MessageRail::Manufacturing,
                    Some(RailAudience::side(alliance)),
                ));
            }

            // ── Repair ──────────────────────────────────────────────────────
            let repair_events = RepairSystem::advance(&mut repair_state, &world, &tick_events);
            for evt in &repair_events {
                if let RepairEvent::ShipRepaired {
                    fleet,
                    ship_index,
                    hull_after,
                    hits_after,
                    ..
                } = evt
                {
                    if let Some(f) = world.fleets.get_mut(*fleet) {
                        if let Some(ship) = f.capital_ships.get_mut(*ship_index) {
                            ship.hull_current = *hull_after;
                            ship.subsystem_hits = *hits_after;
                        }
                    }
                }
            }

            // ── Uprising and disaster (FUN_0050b800, events 0x38d and 0x38f) ──
            let uprising_tick = tick_events.last().map_or(0, |event| event.tick);
            let uprising_budget =
                UprisingSystem::roll_budget(&uprising_state, &world, uprising_tick);
            let uprising_rolls: Vec<f64> = (0..uprising_budget)
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            let uprising_events = UprisingSystem::advance(
                &mut uprising_state,
                &world,
                &economy_state,
                &mission_state,
                &tick_events,
                &uprising_rolls,
            );
            // ── Injury recovery: each healing character's timer 0x388 ───────
            for heal in &rebellion_core::healing::HealingSystem::advance(
                &mut healing_state,
                &world,
                &tick_events,
            ) {
                rebellion_core::healing::apply(&mut world, heal);
                if heal.injury == 0 {
                    push_character_health(&mut msg_log, &world, heal.character, heal.tick);
                }
            }
            // A capture's hook (slot +0x34c, FUN_004f1a80) clears the
            // captive's command (FUN_004eeed0).
            rebellion_core::character_command::release_unfit(&mut world);

            // ── Force discovery: each discovering trainer's timer 0x389 ────
            let discovery_rolls: Vec<f64> = (0
                ..rebellion_core::force_discovery::ForceDiscoverySystem::roll_budget(&world))
                .map(|_| sim_rng.gen::<f64>())
                .collect();
            let discovered = rebellion_core::force_discovery::ForceDiscoverySystem::advance(
                &mut force_discovery_state,
                &world,
                &tick_events,
                &discovery_rolls,
            );
            for found in &discovered {
                rebellion_core::force_discovery::apply(&mut world, found);
            }
            push_force_discoveries(&mut msg_log, &world, &discovered, &message_texts);

            // ── Informants: each held system's timer 0x38e ──────────────────
            let informant_rolls: Vec<f64> =
                (0..rebellion_core::informant::InformantSystem::roll_budget(&world))
                    .map(|_| sim_rng.gen::<f64>())
                    .collect();
            for report in &rebellion_core::informant::InformantSystem::advance(
                &mut informant_state,
                &world,
                &tick_events,
                &informant_rolls,
            ) {
                let recipient_is_alliance = report.recipient == Faction::Alliance;
                if recipient_is_alliance {
                    fog_alliance_state.reveal(report.system);
                } else {
                    fog_empire_state.reveal(report.system);
                }
                let name = world
                    .systems
                    .get(report.system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                // Notification 0x153, Informants Provide Information
                // (FUN_00491060, TEXTSTRA 0x7150-0x7151).
                if let Some((title, text)) =
                    rebellion_render::story_messages::informants_texts(&name, &message_texts)
                {
                    msg_log.push(filed(
                        GameMessage::at_system(
                            report.tick,
                            text,
                            MessageCategory::Event,
                            report.system,
                        )
                        .with_title(title)
                        .with_picture_if(Some(pictures::informants())),
                        MessageRail::PopularSupport,
                        Some(RailAudience::side(recipient_is_alliance)),
                    ));
                }
            }
            for evt in &uprising_events {
                use rebellion_core::uprising::UprisingEvent;
                // Timer 0x384 only sets the support (FUN_0050c9f0); it
                // posts no message of its own.
                if let UprisingEvent::SupportEroded { .. } = evt {
                    rebellion_core::uprising::apply_uprising_event(&mut world, evt);
                    continue;
                }
                // Notification 0x155, New Resources or Resources Depleted
                // (FUN_0048e940, TEXTSTRA 0x7194-0x719a).
                if let UprisingEvent::ResourceChanged {
                    system, tick, code, ..
                } = evt
                {
                    let name = world
                        .systems
                        .get(*system)
                        .map_or_else(|| "unknown".into(), |s| s.name.clone());
                    if let Some((title, text)) =
                        rebellion_render::story_messages::resource_texts(&name, *code, &message_texts)
                    {
                        msg_log.push(filed(
                            GameMessage::at_system(*tick, text, MessageCategory::Event, *system)
                                .with_title(title)
                                .with_picture_if(Some(pictures::system_incident(false))),
                            MessageRail::Resource,
                            system_audience(&world, *system),
                        ));
                    }
                    rebellion_core::uprising::apply_uprising_event(&mut world, evt);
                    continue;
                }
                let (system, tick) = match evt {
                    UprisingEvent::ResourceChanged { system, tick, .. }
                    | UprisingEvent::SupportEroded { system, tick, .. }
                    | UprisingEvent::UprisingBegan { system, tick }
                    | UprisingEvent::UprisingEnded { system, tick }
                    | UprisingEvent::UprisingIncident { system, tick, .. }
                    | UprisingEvent::Disaster { system, tick, .. } => (*system, *tick),
                };
                let name = world
                    .systems
                    .get(system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let holder = world.systems.get(system).and_then(|s| s.control.faction());
                let (mut text, category) = match evt {
                    UprisingEvent::UprisingBegan { .. } => {
                        // A revolt helps the player when the enemy holds the system.
                        let player_gains = world
                            .systems
                            .get(system)
                            .and_then(|s| s.control.faction())
                            .is_some_and(|holder| {
                                (holder == Faction::Alliance)
                                    != (player_faction == MissionFaction::Alliance)
                            });
                        advisor_uprising(&mut advisor_state, &name, player_gains);
                        (format!("Uprising at {name}!"), MessageCategory::Diplomacy)
                    }
                    UprisingEvent::UprisingEnded { .. } => (
                        format!("The uprising at {name} has ended"),
                        MessageCategory::Diplomacy,
                    ),
                    UprisingEvent::UprisingIncident { .. } => (
                        format!("Uprising incident at {name}"),
                        MessageCategory::Diplomacy,
                    ),
                    UprisingEvent::Disaster { .. } => (
                        format!("Natural disaster strikes {name}"),
                        MessageCategory::Event,
                    ),
                    UprisingEvent::SupportEroded { .. } | UprisingEvent::ResourceChanged { .. } => {
                        continue
                    }
                };
                // FUN_00499460 and FUN_0048e940; an incident has no class.
                let picture = match evt {
                    UprisingEvent::UprisingBegan { .. } => Some(pictures::uprising_begins()),
                    UprisingEvent::UprisingEnded { .. } => {
                        pictures::uprising_ends(pictures::system_side(&world, system))
                    }
                    UprisingEvent::UprisingIncident { .. }
                    | UprisingEvent::SupportEroded { .. }
                    | UprisingEvent::ResourceChanged { .. } => None,
                    UprisingEvent::Disaster { .. } => Some(pictures::system_incident(true)),
                };
                // The original texts: Uprising Begins / Ends (0x14b,
                // FUN_00499460) for both sides, Natural Disaster (0x154,
                // FUN_0048e940) for the holder. port: a disaster on a neutral
                // system keeps the port's line, off the Message Index.
                let mut title = None;
                let mut audience = system_audience(&world, system);
                let original = match evt {
                    UprisingEvent::UprisingBegan { .. } | UprisingEvent::UprisingEnded { .. } => {
                        audience = holder
                            .filter(|side| *side != Faction::Neutral)
                            .map(|_| RailAudience::Both);
                        rebellion_render::story_messages::uprising_texts(
                            &name,
                            holder,
                            matches!(evt, UprisingEvent::UprisingBegan { .. }),
                            &message_texts,
                        )
                    }
                    UprisingEvent::Disaster {
                        total_energy,
                        raw_materials,
                        destroyed,
                        ..
                    } if audience.is_some() => {
                        let units: Vec<String> = destroyed
                            .iter()
                            .filter_map(|&facility| world.facility(facility))
                            .filter_map(|(class, _)| {
                                world.buildable_classes.get(&class).map(|c| c.name.clone())
                            })
                            .collect();
                        let resources = world.systems.get(system).is_some_and(|s| {
                            *total_energy < s.total_energy || *raw_materials < s.raw_materials
                        });
                        let texts = rebellion_render::story_messages::disaster_texts(
                            &name,
                            resources,
                            &units,
                            &message_texts,
                        );
                        if texts.is_none() && !message_texts.is_empty() {
                            // Nothing lost: the class stays silent.
                            rebellion_core::uprising::apply_uprising_event(&mut world, evt);
                            continue;
                        }
                        texts
                    }
                    // Notification 0x152 (SystemUprisingIncidentNotif,
                    // FUN_00512580) maps to no message type (FUN_00488de0):
                    // port: the incident keeps its line in the log, off the
                    // Message Index.
                    UprisingEvent::UprisingIncident { .. } => {
                        audience = None;
                        None
                    }
                    _ => None,
                };
                if let Some((original_title, body)) = original {
                    title = Some(original_title);
                    text = body;
                }
                // Notification 3, Uprising Message.
                let mut message =
                    GameMessage::at_system(tick, text, category, system).with_picture_if(picture);
                if let Some(title) = title {
                    message = message.with_title(title);
                }
                msg_log.push(filed(message, MessageRail::PopularSupport, audience));
                rebellion_core::uprising::apply_uprising_event(&mut world, evt);
                if let UprisingEvent::UprisingIncident { losses, .. } = evt {
                    for loss in losses {
                        if let rebellion_core::uprising::IncidentLoss::CharacterInjured {
                            character,
                            ..
                        } = loss
                        {
                            push_character_health(&mut msg_log, &world, *character, tick);
                        }
                    }
                }
            }

            // ── (The port's Jedi tiers are gone: the Force grows through the
            // missions, Dagobah, escapes and the encounter;
            // ghidra/notes/jedi-rs-replacement.md.)

            // ── Loyalty: Coruscant taken, the Alliance HQ lost ──────────────
            // (FUN_005616a0, FUN_00561780; ghidra/notes/betrayal-and-loyalty.md)
            rebellion_core::loyalty::settle(
                &mut world,
                loyalty_before,
                Some(victory_state.alliance_hq),
                &mut |m| sim_rng.gen_range(0..m.max(1)),
            );

            // ── Victory check ────────────────────────────────────────────────
            // The victory conditions word (FUN_005180e0): each change is
            // 0x128, Alliance Headquarters Destroyed (FUN_0048e5a0) with
            // bit 0x1 and Coruscant Captured (FUN_0048f790) while the
            // Alliance holds Coruscant.
            if !tick_events.is_empty() {
                if let Some((_, new)) = rebellion_core::victory::advance_conditions_word(
                    &mut victory_state,
                    &world,
                    campaign_config.victory_conditions,
                ) {
                    for message in rebellion_render::story_messages::victory_conditions_messages(
                        new,
                        current_tick,
                        &message_texts,
                    ) {
                        msg_log.push(message);
                    }
                }
            }
            if let Some(outcome) = VictorySystem::check(
                &victory_state,
                &world,
                &tick_events,
                campaign_config.victory_conditions,
            ) {
                victory_state.resolved = true;
                let msg = match &outcome {
                    rebellion_core::victory::VictoryOutcome::HqCaptured {
                        winner, loser, ..
                    } => {
                        format!(
                            "{winner:?} captured {loser:?} headquarters! {winner:?} wins!"
                        )
                    }
                    rebellion_core::victory::VictoryOutcome::HqDestroyed { .. } => {
                        "The Empire destroyed the Alliance headquarters and secured its system. Empire wins!"
                            .to_string()
                    }
                    rebellion_core::victory::VictoryOutcome::DeathStarVictory { .. } => {
                        "The Death Star destroyed the Alliance headquarters system. Empire wins!"
                            .to_string()
                    }
                };
                msg_log.push(GameMessage::new(current_tick, msg, MessageCategory::Event));

                game_mode = GameMode::VictoryModal {
                    alliance_won: victory_winner_is_alliance(&outcome),
                };
            }
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

        // ── Rendering (mode-specific) ────────────────────────────────────────

        match game_mode {
            GameMode::Cutscene { ref kind } => {
                clear_background(BLACK);

                let next_mode = match kind {
                    CutsceneKind::Intro => GameMode::MainMenu,
                    CutsceneKind::TrenchRun(_) => GameMode::TacticalCombat,
                    CutsceneKind::Story(_) => GameMode::Galaxy,
                };
                #[cfg(not(target_arch = "wasm32"))]
                let next_music = music_context_after_cutscene(kind);

                if let Some(player) = cutscene_player.as_mut() {
                    player.advance(dt);
                    if let Some(frame) = player.current_frame() {
                        draw_fullscreen_texture(frame);
                    }

                    let skip_label = "SPACE / ESC to skip";
                    let metrics = measure_text(skip_label, None, 24, 1.0);
                    draw_text(
                        skip_label,
                        screen_width() - metrics.width - 24.0,
                        screen_height() - 24.0,
                        24.0,
                        Color::new(1.0, 1.0, 1.0, 0.8),
                    );

                    if player.is_finished() {
                        cutscene_player = None;
                        game_mode = next_mode;
                        #[cfg(not(target_arch = "wasm32"))]
                        audio_engine.play_music_for_context(next_music, &sounds_dir, &audio_vol);
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
                #[cfg(target_arch = "wasm32")]
                if browser_menu_audio_requested {
                    if let Some(engine) = browser_menu_audio.as_mut() {
                        engine.try_play_loaded_music(&audio_vol);
                    }
                }

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
                        MainMenuAction::Credits => {
                            credits_state.reset();
                            game_mode = GameMode::Credits;
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

            GameMode::Credits => {
                clear_background(BLACK);
                let mut destination_action = None;
                egui_macroquad::ui(|ctx| {
                    destination_action = draw_credits(ctx, &mut credits_state);
                });
                egui_macroquad::draw();
                if destination_action == Some(MenuDestinationAction::Back) {
                    game_mode = GameMode::MainMenu;
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

            GameMode::LoadGame => {
                clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
                egui_macroquad::ui(|ctx| {
                    egui_macroquad::egui::TopBottomPanel::bottom("main_menu_audio_options").show(
                        ctx,
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Audio");
                                draw_audio_controls(ui, &mut audio_vol);
                            });
                        },
                    );
                    if let Some(action) =
                        draw_save_load(ctx, &save_slots, &mut save_load_panel_state)
                    {
                        panel_actions.push(action);
                    }
                });
                egui_macroquad::draw();
            }

            GameMode::GameOptions { origin } => {
                let control =
                    is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                let legacy_load =
                    is_key_pressed(KeyCode::F8) || (control && is_key_pressed(KeyCode::L));
                let legacy_save =
                    is_key_pressed(KeyCode::F9) || (control && is_key_pressed(KeyCode::S));
                if game_options_state.pending.is_none()
                    && origin != GameOptionsOrigin::TacticalBattle
                    && (legacy_load || (legacy_save && origin == GameOptionsOrigin::CommandCenter))
                {
                    if legacy_save {
                        save_load_panel_state.open_save();
                    } else {
                        save_load_panel_state.open_load();
                    }
                    game_options_state.suspended = true;
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
                    if game_options_state.suspended {
                        if let Some(action) =
                            draw_save_load(ctx, &save_slots, &mut save_load_panel_state)
                        {
                            panel_actions.push(action);
                        }
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
                        macroquad::logging::info!("[game_options] command=restart");
                    }
                    GameOptionsAction::Exit => quit_requested = true,
                    GameOptionsAction::Delete { slot } => {
                        panel_actions.push(PanelAction::DeleteSave { slot })
                    }
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
                                #[cfg(target_arch = "wasm32")]
                                rng_seed: None,
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
                                    campaign_generation += 1;
                                    sim_rng = Xoshiro256PlusPlus::seed_from_u64(
                                        rng_seed.wrapping_add(u64::from(campaign_generation)),
                                    );
                                    clock = GameClock::new();
                                    game_speed_ui = GameSpeedUiState::default();
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
                                    game_config = rebellion_core::tuning::GameConfig::default();
                                    dual_ai_mode = false;
                                    secondary_ai_state = None;

                                    map_state = GalaxyMapState::default();
                                    warmed_galaxy_font_sizes.clear();
                                    msg_log = MessageLog::default();
                                    log_state = MessageLogState::default();
                                    build_selection_state = BuildSelectionState::default();
                                    mission_dialog_state = MissionDialogState::default();
                                    move_confirmation_state = MoveConfirmationState::default();
                                    status_window_state = StatusWindowState::default();
                                    fleet_finder_state = FleetFinderState::default();
                                    troop_finder_state = TroopFinderState::default();
                                    personnel_finder_state = PersonnelFinderState::default();
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
                                    show_save_load = false;
                                    save_load_panel_state =
                                        rebellion_render::SaveLoadPanelState::default();
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

                                // AI controls the opposite faction
                                if faction == MissionFaction::Empire {
                                    ai_state = AIState::new(AiFaction::Alliance);
                                } else {
                                    ai_state = AIState::new(AiFaction::Empire);
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

                                // Start galaxy map music and play faction voice greeting.
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    audio_engine.play_music_for_context(
                                        MusicContext::GalaxyMap,
                                        &sounds_dir,
                                        &audio_vol,
                                    );
                                }

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
                map_state.pointer_blocked = sector_window_state
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
                    || pause_alert_contains_screen_point(&clock, cockpit_layout, pointer);
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
                // messages for the player's side (FUN_0048a2a0).
                cockpit_state.message_unread_mask =
                    msg_log.unread_mask(player_faction == MissionFaction::Alliance);

                // 4. All egui panels in a single ui() + draw() pass
                egui_macroquad::ui(|ctx| {
                    let strategic_input_enabled = !original_modal_fixture_open;
                    // Register the cockpit background before panels so the
                    // opaque chrome never covers their content or artwork.
                    draw_cockpit_background(ctx, &cockpit_state, &mut bmp_cache);
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

                    draw_gid_caption(
                        ctx,
                        galaxy_camera,
                        cockpit_state.faction,
                        cockpit_state.gid_mode,
                    );
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
                            &blockade_state,
                            &uprising_state,
                            cockpit_state.faction,
                        ),
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
                    if speed_input {
                        rebellion_render::agent_menu::open_agent_menu_on_right_click(
                            ctx,
                            &mut agent_menu_state,
                            cockpit_layout,
                            cockpit_state.faction,
                        );
                    }
                    let agent_view = rebellion_render::agent_menu::AgentMenuView {
                        garrisons: player_agent.is_running(AutomationModule::Garrisons),
                        production: player_agent.is_running(AutomationModule::Production),
                        advice: player_agent.advice(),
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
                        apply_agent_command(&mut player_agent, command, "menu");
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
                    if draw_pause_alert(ctx, &clock, &mut bmp_cache, cockpit_layout, speed_input) {
                        clock.resume();
                    }

                    // Save/Load panel (floating window)
                    if show_save_load {
                        save_load_panel_state.open = true;
                        if let Some(action) =
                            draw_save_load(ctx, &save_slots, &mut save_load_panel_state)
                        {
                            match &action {
                                PanelAction::CloseSaveLoadPanel => {
                                    show_save_load = false;
                                }
                                _ => {
                                    panel_actions.push(action);
                                }
                            }
                        }
                    }

                    // Encyclopedia (floating window)
                    if let Some(sys_key) =
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

                    for action in draw_sector_windows(
                        ctx,
                        &world,
                        &movement_state,
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
                                fleet_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SectorWindowAction::OpenDefensesWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
                                defenses_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
                            }
                            SectorWindowAction::OpenMissionsWindow {
                                system,
                                logical_position,
                            } => {
                                map_state.selected_system = Some(system);
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
                            SectorWindowAction::OpenObjectMenu { selection, point } => {
                                let mut gates = order_gates(
                                    selection,
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
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
                            // 4 entries apart (0x214 against +0x68). port: only
                            // the fleet icon's drop moves; its fleets are the
                            // system's of the player's side (FUN_00512700).
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
                            SectorWindowAction::DropSelection { .. } => {}
                        }
                    }

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
                            } => {
                                defenses_window_state.open(
                                    &world,
                                    system,
                                    logical_position,
                                    cockpit_state.faction,
                                    cockpit_layout,
                                );
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
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                );
                                object_menu = Some(ObjectMenuState::new(selection, gates, point));
                            }
                        }
                    }

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
                            FleetWindowAction::Rename { entry, name } => {
                                panel_actions.push(PanelAction::Rename {
                                    fleet: entry.fleet(),
                                    ship: match entry {
                                        rebellion_render::fleet_window::FleetWindowEntry::Ship { index, .. } => Some(index),
                                        rebellion_render::fleet_window::FleetWindowEntry::Fleet(_) => None,
                                    },
                                    name,
                                });
                            }
                            FleetWindowAction::OpenObjectMenu { selection, point } => {
                                let gates = order_gates(
                                    Some(selection),
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                );
                                object_menu =
                                    Some(ObjectMenuState::new(Some(selection), gates, point));
                            }
                            // FUN_00422ce0, window type 4: the fleet moves
                            // with 0x201 against what +0x70 gives under the
                            // point. port: Ctrl's 0x202 is not ported here.
                            FleetWindowAction::DragFleet { fleet, point } => {
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
                                    issue_fleet_move(
                                        &FleetMoveContext {
                                            world: &world,
                                            movement: &movement_state,
                                            blockaded: blockade_state.blockaded_systems(),
                                            faction: player_faction,
                                            tick: clock.tick,
                                        },
                                        &[fleet],
                                        false,
                                        target,
                                        &mut msg_log,
                                        &mut move_confirmation_state,
                                        &mut panel_actions,
                                    );
                                }
                            }
                            // FUN_00422ce0: the ship moves with 0x201 against
                            // the window under the point
                            // (ghidra/notes/fleet-join-split.md).
                            FleetWindowAction::DragShip {
                                fleet,
                                index,
                                roster,
                                point,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                match release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    Some(ReleaseTarget::Fleet { fleet: target, .. }) => {
                                        panel_actions.push(PanelAction::JoinFleet {
                                            mover: FleetMover::Ships {
                                                fleet,
                                                ships: vec![index],
                                                roster,
                                            },
                                            target,
                                        });
                                    }
                                    Some(ReleaseTarget::System(system)) => {
                                        panel_actions.push(PanelAction::MoveShips {
                                            fleet,
                                            ships: vec![index],
                                            roster,
                                            system,
                                        });
                                    }
                                    None => {}
                                }
                            }
                            FleetWindowAction::DragRegiment { troop, point } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let target = match release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    Some(ReleaseTarget::System(system)) => {
                                        RegimentTarget::System(system)
                                    }
                                    Some(ReleaseTarget::Fleet { fleet, .. }) => {
                                        RegimentTarget::Fleet(fleet)
                                    }
                                    None => continue,
                                };
                                panel_actions.push(PanelAction::MoveRegiment { troop, target });
                            }
                            // FUN_00422ce0: the character's 0x201 -> 0x241
                            // against the window under the point.
                            FleetWindowAction::DragCharacter { character, point } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let action = match release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    Some(ReleaseTarget::System(system)) => {
                                        PanelAction::MoveCharacter {
                                            character,
                                            system,
                                            fleet: None,
                                        }
                                    }
                                    Some(ReleaseTarget::Fleet { fleet, system }) => {
                                        PanelAction::MoveCharacter {
                                            character,
                                            system,
                                            fleet: Some(fleet),
                                        }
                                    }
                                    None => continue,
                                };
                                panel_actions.push(action);
                            }
                            // FUN_00422ce0: the squadron's 0x201 against the
                            // window under the point.
                            FleetWindowAction::DragSquadron {
                                fleet,
                                index,
                                point,
                            } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let Some(class) = world
                                    .fleets
                                    .get(fleet)
                                    .and_then(|value| value.fighters.get(index))
                                    .map(|entry| entry.class)
                                else {
                                    continue;
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
                                panel_actions.push(PanelAction::MoveSquadron {
                                    class,
                                    from: SquadronPlace::Fleet(fleet),
                                    to: target.squadron_place(),
                                });
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
                            } => system_window_state
                                .minimize_defenses_window(system, logical_position),
                            DefensesWindowAction::OpenObjectMenu {
                                selection, point, ..
                            } => {
                                let gates = order_gates(
                                    selection,
                                    &world,
                                    &mission_state,
                                    &movement_state,
                                    &troop_transport_state,
                                    &mfg_state,
                                    &research_state,
                                    player_faction,
                                );
                                object_menu = Some(ObjectMenuState::new(selection, gates, point));
                            }
                            // FUN_00422ce0, window type 10: the character's
                            // 0x201 -> 0x241 against the window under the
                            // point.
                            DefensesWindowAction::DragCharacter { character, point } => {
                                let windows = ReleaseWindows {
                                    sector: &sector_window_state,
                                    system: &system_window_state,
                                    fleet: &fleet_window_state,
                                    defenses: &defenses_window_state,
                                    missions: &missions_window_state,
                                };
                                let (system, fleet) = match release_destination(
                                    ctx,
                                    &world,
                                    &movement_state,
                                    fog_state,
                                    cockpit_layout,
                                    windows,
                                    point,
                                ) {
                                    Some(ReleaseTarget::System(system)) => (system, None),
                                    Some(ReleaseTarget::Fleet { fleet, system }) => {
                                        (system, Some(fleet))
                                    }
                                    None => continue,
                                };
                                panel_actions.push(PanelAction::MoveCharacter {
                                    character,
                                    system,
                                    fleet,
                                });
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
                        }
                    }

                    // The menu commits on the release that starts targeting;
                    // only a later release may end the order.
                    let targeting_held = targeting.is_some();
                    match draw_object_menu(
                        ctx,
                        &mut object_menu,
                        &mut bmp_cache,
                        cockpit_layout,
                        cockpit_state.faction,
                        strategic_input_enabled,
                    ) {
                        // FUN_00486fb0 0x234: Planetary Assault of the
                        // selection's system by its fleets.
                        Some((ObjectMenuCommand::Assault, selection)) => {
                            if let Some((fleets, system)) = bombard_team(
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
                        Some((ObjectMenuCommand::Encyclopedia, _)) => enc_state.open = true,
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
                                            blockaded: false,
                                            destination: SystemKey::default(),
                                            join: None,
                                            lines: vec![(name, 0)],
                                        });
                                    }
                                    Some(Err(_)) => {
                                        msg_log.push(GameMessage::new(
                                            clock.tick,
                                            "Retire refused".to_string(),
                                            MessageCategory::Event,
                                        ));
                                    }
                                    None => {}
                                }
                            }
                        }
                        // FUN_00487740: the Abort validates (FUN_00521600),
                        // then FUN_00487cc0 asks (FUN_0049a350, case 0x250).
                        Some((ObjectMenuCommand::Abort, selection @ Some(_))) => {
                            let missions = abort_missions(
                                &world,
                                fog_state,
                                &mission_state,
                                selection,
                                player_faction,
                            );
                            macroquad::logging::info!(
                                "[interface] command=0x250 missions={missions:?}"
                            );
                            if !missions.is_empty() {
                                move_confirmation_state.open(MoveConfirmation {
                                    order: ConfirmedOrder::Abort,
                                    faction: player_faction,
                                    fleets: Vec::new(),
                                    characters: Vec::new(),
                                    special_forces: Vec::new(),
                                    missions,
                                    blockaded: false,
                                    destination: SystemKey::default(),
                                    join: None,
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
                                    &world,
                                    system,
                                    player_faction == MissionFaction::Alliance,
                                ),
                                confirmed: command == ObjectMenuCommand::ConfirmedMove,
                            }));
                        }
                        Some((ObjectMenuCommand::Move, Some(MenuObject::Troop(troop)))) => {
                            targeting = Some(Targeting::new(TargetOrder::TroopMove { troop }));
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
                            ObjectMenuCommand::Move,
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
                                }));
                            }
                        }
                        Some((
                            ObjectMenuCommand::Move,
                            Some(MenuObject::PlanetFighter { system, class }),
                        )) => {
                            targeting = Some(Targeting::new(TargetOrder::SquadronMove {
                                class,
                                from: SquadronPlace::Planet(system),
                            }));
                        }
                        Some((
                            ObjectMenuCommand::Move,
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
                            let kinds = rebellion_core::build_selection::listed_classes(
                                &world,
                                &research_state,
                                area,
                                is_alliance,
                            );
                            if build_selection_state.open(system, area, is_alliance, kinds) {
                                macroquad::logging::info!(
                                    "[interface] command=0x21x destination=build_selection status=opened area={area:?} system={}",
                                    world.systems.get(system).map_or("", |value| value.name.as_str())
                                );
                            }
                        }
                        // Stop (0x213) runs at once: the band's area drops
                        // what it was building (manual p. 84).
                        Some((
                            ObjectMenuCommand::Stop,
                            Some(MenuObject::Producer { system, area }),
                        )) => panel_actions.push(PanelAction::StopProduction { system, area }),
                        // FUN_00486fb0 0x203 → FUN_0041d600 → FUN_00429350:
                        // the edit opens in the window that lists the object.
                        // port: with no Fleet window open there, one opens.
                        Some((
                            ObjectMenuCommand::Rename,
                            Some(object @ (MenuObject::Fleet(_) | MenuObject::Ship { .. })),
                        )) => {
                            let entry = match object {
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
                        &mut build_selection_state,
                        cockpit_layout,
                        &mut bmp_cache,
                    ) {
                        Some(BuildSelectionAction::Confirm {
                            system,
                            area,
                            kind,
                            count,
                        }) => panel_actions.push(PanelAction::BuildProduction {
                            system,
                            area,
                            kind,
                            count,
                        }),
                        Some(BuildSelectionAction::Encyclopedia) => enc_state.open = true,
                        None => {}
                    }
                    #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
                    if let Some(request) = interface_fixture_request {
                        interface_test_fixture::emit_production(
                            request,
                            &world,
                            &mfg_state,
                            &system_window_state,
                            &build_selection_state,
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
                        if let Some(system) = rebellion_core::movement::listed_location(
                            &movement_state,
                            &world,
                            entry.fleet(),
                        ) {
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
                        &mut troop_finder_state,
                        cockpit_layout,
                        &mut bmp_cache,
                        &troop_transport_state,
                    )
                    .map(|action| match action {
                        TroopFinderAction::OpenDefenses { system } => {
                            (system, FinderTarget::Defenses)
                        }
                        TroopFinderAction::OpenFleet { system, fleet } => {
                            (system, FinderTarget::Fleet(fleet))
                        }
                    });
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
                        PersonnelFinderAction::OpenSystem { system } => {
                            Some((system, FinderTarget::Sector))
                        }
                        PersonnelFinderAction::OpenFleet { system, fleet } => {
                            Some((system, FinderTarget::Fleet(fleet)))
                        }
                    });
                    if let Some((system, target)) = troop_choice.or(personnel_choice) {
                        sector_window_state.open_for_system(&world, system, cockpit_state.faction);
                        map_state.selected_system = Some(system);
                        match target {
                            FinderTarget::Sector => {}
                            FinderTarget::Defenses => {
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
                        }
                    }

                    // FUN_004443a0: 0x66 closes the Status window and opens
                    // the Encyclopedia; 0x65 closes it.
                    if let Some(action) = draw_status_window(
                        ctx,
                        StatusSources {
                            world: &world,
                            missions: &mission_state,
                            movement: &movement_state,
                            transport: &troop_transport_state,
                            manufacturing: &mfg_state,
                            today: clock.tick,
                        },
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
                    if let Some(MoveConfirmationAction::Abort { missions }) = &answer {
                        for &id in missions {
                            let aborted = mission_state.abort_allowed(id, player_faction)
                                && mission_state.abort(id);
                            macroquad::logging::info!(
                                "[interface] command=0x250 destination=abort mission={id} result={aborted}"
                            );
                        }
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
                        destination,
                        join,
                    }) = answer
                    {
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
                                    mission_dialog_state.open(player_faction, target, team, kinds);
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
                                    order: TargetOrder::FleetMove { fleets, confirmed },
                                    target,
                                }) => issue_fleet_move(
                                    &FleetMoveContext {
                                        world: &world,
                                        movement: &movement_state,
                                        blockaded: blockade_state.blockaded_systems(),
                                        faction: player_faction,
                                        tick: clock.tick,
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
                                    if confirmed {
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
                                    if confirmed {
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
                                // A regiment's 0x201 against the release
                                // window's +0x70 (FUN_004a3130 for a Fleet
                                // window), boarding, landing or travelling.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::TroopMove { troop },
                                    target: ReleaseTarget::Fleet { fleet, .. },
                                }) => panel_actions.push(PanelAction::MoveRegiment {
                                    troop,
                                    target: RegimentTarget::Fleet(fleet),
                                }),
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::TroopMove { troop },
                                    target: ReleaseTarget::System(system),
                                }) => panel_actions.push(PanelAction::MoveRegiment {
                                    troop,
                                    target: RegimentTarget::System(system),
                                }),
                                // A squadron's 0x201 against the release
                                // window's +0x70: boarding or landing.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::SquadronMove { class, from },
                                    target,
                                }) => panel_actions.push(PanelAction::MoveSquadron {
                                    class,
                                    from,
                                    to: target.squadron_place(),
                                }),
                                // FUN_00512700 kind 4: the order's team is
                                // every production area of the system, or
                                // a band's own area.
                                Some(TargetingEnd::Target {
                                    order: TargetOrder::Destination { system, area },
                                    target:
                                        ReleaseTarget::System(destination)
                                        | ReleaseTarget::Fleet {
                                            system: destination,
                                            ..
                                        },
                                }) => panel_actions.push(PanelAction::SetDestination {
                                    system,
                                    area,
                                    destination,
                                }),
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
                    let cockpit_command = (!original_modal_fixture_open && targeting.is_none())
                        .then(|| {
                            handle_cockpit_egui_input(ctx, &mut cockpit_state, &mut bmp_cache)
                        });
                    if let Some(btn) = cockpit_command.flatten() {
                        let (command, destination) = match btn {
                            CockpitButton::SystemFinder => (0x12d, "system_finder"),
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
            }

            GameMode::TacticalCombat => {
                let tac_action = draw_tactical_view(
                    &mut tactical_state,
                    &mut bmp_cache,
                    &world,
                    &troop_transport_state,
                );
                let requested_result_system = match &tac_action {
                    TacticalAction::OpenBattleSystem(system) => Some(*system),
                    _ => None,
                };
                let requested_result_fleet = match &tac_action {
                    TacticalAction::OpenBattleFleet(fleet) => Some(*fleet),
                    _ => None,
                };

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
                            let (space_result, _) =
                                rebellion_core::abstract_battle::resolve_space_battle(
                                    &world,
                                    session.system,
                                    &[session.attacker_fleet],
                                    &[session.defender_fleet],
                                    rebellion_core::abstract_battle::abstract_seed(&combat_rolls),
                                    session.start_tick,
                                    retreat_allowed,
                                );
                            apply_space_combat_result(&space_result, &mut world);
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
                            rebellion_core::abstract_battle::withdraw_loser(
                                &mut world,
                                &mut movement_state,
                                &mut troop_transport_state.squadrons,
                                session.system,
                                winner,
                                clock.tick,
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
                    TacticalAction::ReturnToGalaxy
                    | TacticalAction::OpenBattleSystem(_)
                    | TacticalAction::OpenBattleFleet(_) => {
                        let strategic_results_applied = tactical_state.strategic_results_applied();
                        // Apply combat results from tactical session to GameWorld.
                        if let Some(session) = tactical_state.end_battle() {
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
                                rebellion_core::abstract_battle::withdraw_loser(
                                    &mut world,
                                    &mut movement_state,
                                    &mut troop_transport_state.squadrons,
                                    session.system,
                                    winner,
                                    clock.tick,
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
                            if strategic_results_applied {
                                game_mode = GameMode::Galaxy;
                            } else {
                                game_mode = GameMode::Galaxy;
                            }
                        } else {
                            game_mode = GameMode::Galaxy;
                        }

                        if game_mode == GameMode::Galaxy {
                            if let Some(system) = requested_result_system {
                                map_state.selected_system = Some(system);
                                let layout = cockpit_state.layout();
                                let _ = system_window_state.open(
                                    &world,
                                    system,
                                    (85, 55),
                                    cockpit_state.faction,
                                    layout,
                                );
                            }
                            // port: the fleet's Fleet window with the fleet
                            // selected, as the Fleet Finder opens it
                            // (FUN_00429440).
                            if let Some(fleet) = requested_result_fleet {
                                if let Some(value) = world.fleets.get(fleet) {
                                    let system = value.location;
                                    map_state.selected_system = Some(system);
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
                                    }
                                }
                            }
                        }
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
                    TacticalAction::RetreatSelected => {
                        if let Some(ref mut session) = tactical_state.session {
                            let player_side = session.player_is_attacker;
                            for ship in &mut session.ships {
                                if ship.selected && ship.is_attacker == player_side && ship.alive {
                                    ship.retreating = true;
                                    ship.selected = false;
                                }
                            }
                            session.selected_ship = None;
                        }
                    }
                    TacticalAction::WithdrawFromBattle => {
                        if let Some(ref mut session) = tactical_state.session {
                            let player_side = session.player_is_attacker;
                            for ship in &mut session.ships {
                                if ship.alive
                                    && !ship.retreating
                                    && ship.is_attacker == player_side
                                    && ship.subsystem_capacity.hyperdrive > 0
                                    && ship.subsystem_condition.hyperdrive > 0
                                {
                                    ship.retreating = true;
                                    ship.selected = false;
                                }
                            }
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
                            &mut msg_log,
                            clock.tick,
                            #[cfg(not(target_arch = "wasm32"))]
                            &mut audio_engine,
                        );
                        if cutscene_player.is_some() {
                            game_mode = GameMode::Cutscene {
                                kind: CutsceneKind::TrenchRun(outcome),
                            };
                        }
                    }
                }
            }

            GameMode::VictoryModal { alliance_won } => {
                // Frozen galaxy backdrop with modal overlay.
                clear_background(Color::new(0.02, 0.02, 0.06, 1.0));
                egui_macroquad::ui(|ctx| {
                    use egui_macroquad::egui;
                    let title = if alliance_won {
                        "Alliance Victory!"
                    } else {
                        "Imperial Victory!"
                    };
                    let body = if alliance_won {
                        "The Rebel Alliance has triumphed. Freedom is restored to the galaxy."
                    } else {
                        "The Galactic Empire has crushed the Rebellion. Order reigns supreme."
                    };
                    egui::Area::new(egui::Id::new("victory_modal"))
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgba_unmultiplied(10, 15, 30, 230))
                                .inner_margin(egui::Margin::same(32))
                                .show(ui, |ui| {
                                    ui.set_width(400.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(title)
                                                .size(24.0)
                                                .color(egui::Color32::from_rgb(230, 200, 100)),
                                        );
                                        ui.add_space(16.0);
                                        ui.label(
                                            egui::RichText::new(body)
                                                .size(14.0)
                                                .color(egui::Color32::from_rgb(200, 195, 185)),
                                        );
                                        ui.add_space(24.0);
                                        if ui.button("Continue").clicked() {
                                            game_mode = GameMode::MainMenu;
                                        }
                                    });
                                });
                        });
                });
                egui_macroquad::draw();
            }
        }

        // 5. Apply panel actions
        for action in panel_actions {
            if matches!(game_mode, GameMode::GameOptions { .. }) && game_options_state.suspended {
                let intent = match &action {
                    PanelAction::SaveGame { slot, name } => Some(GameOptionsAction::Save {
                        slot: *slot,
                        name: name.clone(),
                    }),
                    PanelAction::LoadGame { slot } => Some(GameOptionsAction::Load { slot: *slot }),
                    PanelAction::DeleteSave { slot } => {
                        Some(GameOptionsAction::Delete { slot: *slot })
                    }
                    _ => None,
                };
                if let Some(intent) = intent {
                    game_options_state.suspended = false;
                    if game_options_state.request(intent) == GameOptionsAction::None {
                        continue;
                    }
                }
            }
            match action {
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
                        game_config: &mut game_config,
                        campaign_config: &mut campaign_config,
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
                            save_load_panel_state.error_message = None;
                            save_slots = read_save_slots(&saves_dir);
                            game_options_state.refresh_saves(&save_slots);
                            game_options_state.error = None;
                            msg_log.push(GameMessage::new(
                                clock.tick,
                                format!("Saved game to slot {}", slot + 1),
                                MessageCategory::Event,
                            ));
                        }
                        Err(error) => {
                            save_load_panel_state.error_message = Some(error.to_string());
                            game_options_state.error = Some(error.to_string());
                        }
                    }
                }
                PanelAction::LoadGame { slot } => {
                    match rebellion_data::save::load_slot(&saves_dir, slot) {
                        Ok((meta, state)) => {
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
                                game_config: &mut game_config,
                                campaign_config: &mut campaign_config,
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
                            map_state = GalaxyMapState::default();
                            sector_window_state.clear();
                            system_window_state.clear();
                            fleet_window_state.clear();
                            defenses_window_state.clear();
                            assault_summary_state.clear();
                            missions_window_state.clear();
                            object_menu = None;
                            targeting = None;
                            build_selection_state = BuildSelectionState::default();
                            mission_dialog_state = MissionDialogState::default();
                            move_confirmation_state = MoveConfirmationState::default();
                            status_window_state = StatusWindowState::default();
                            fleet_finder_state = FleetFinderState::default();
                            troop_finder_state = TroopFinderState::default();
                            personnel_finder_state = PersonnelFinderState::default();
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
                            show_save_load = false;
                            save_load_panel_state.close();
                            tactical_state = TacticalState::new();
                            dual_ai_mode = secondary_ai_state.is_some();
                            msg_log = MessageLog::default();
                            msg_log.push(GameMessage::new(
                                clock.tick,
                                format!("Loaded save ‘{}’ from slot {}", meta.name, slot + 1),
                                MessageCategory::Event,
                            ));
                            game_mode = GameMode::Galaxy;
                            if let Ok(fingerprint) = restored_fingerprint {
                                macroquad::logging::info!(
                                    "[campaign] restored slot={} mode=Galaxy tick={} fingerprint={} faction={:?}",
                                    slot, clock.tick, fingerprint, player_faction
                                );
                            }
                        }
                        Err(error) => {
                            save_load_panel_state.error_message = Some(error.to_string());
                            game_options_state.error = Some(error.to_string());
                        }
                    }
                }
                PanelAction::DeleteSave { slot } => {
                    match rebellion_data::save::delete_slot(&saves_dir, slot) {
                        Ok(()) => {
                            save_load_panel_state.selected_slot = None;
                            save_load_panel_state.error_message = None;
                            save_slots = read_save_slots(&saves_dir);
                            game_options_state.refresh_saves(&save_slots);
                            game_options_state.error = None;
                            msg_log.push(GameMessage::new(
                                clock.tick,
                                format!("Deleted save in slot {}", slot + 1),
                                MessageCategory::Event,
                            ));
                        }
                        Err(error) => {
                            save_load_panel_state.error_message = Some(error.to_string());
                            game_options_state.error = Some(error.to_string());
                        }
                    }
                }
                PanelAction::SetGameSpeed(speed) => choose_game_speed(&mut clock, speed),
                PanelAction::CloseSaveLoadPanel => {
                    game_options_state.suspended = false;
                    save_load_panel_state.close();
                    show_save_load = false;
                    if game_mode == GameMode::LoadGame {
                        game_mode = GameMode::MainMenu;
                    }
                }
                action => {
                    let active_fog_state = if player_faction == MissionFaction::Alliance {
                        &mut fog_alliance_state
                    } else {
                        &mut fog_empire_state
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
                        #[cfg(not(target_arch = "wasm32"))]
                        &mut audio_engine,
                        #[cfg(not(target_arch = "wasm32"))]
                        &audio_vol,
                        #[cfg(not(target_arch = "wasm32"))]
                        &sounds_dir,
                    );
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
        let screen_music = if game_mode == GameMode::TacticalCombat {
            tactical_state.screen_music()
        } else {
            None
        };
        let tactical_audio_context = game_mode == GameMode::TacticalCombat
            && !tactical_state.battle_alert_open()
            && screen_music.is_none();
        if tactical_audio_context && !tactical_music_active {
            #[cfg(not(target_arch = "wasm32"))]
            audio_engine.play_music_for_context(
                rebellion_render::MusicContext::Combat,
                &sounds_dir,
                &audio_vol,
            );
            #[cfg(target_arch = "wasm32")]
            if let (Some(engine), Some(bytes)) =
                (browser_menu_audio.as_mut(), browser_battle_theme.as_deref())
            {
                engine.load_music_bytes(rebellion_render::MusicTrack::Battle, bytes);
            }
            macroquad::logging::info!(
                "[audio] context=combat track=Battle mdata=307 loaded={} muted={}",
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        original_game_dir().join("MDATA/MDATA.307").exists()
                            || sounds_dir.join("music/battle.wav").exists()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        browser_battle_theme.is_some()
                    }
                },
                audio_vol.muted || audio_vol.music_muted
            );
            tactical_music_active = true;
        } else if !tactical_audio_context && tactical_music_active {
            #[cfg(not(target_arch = "wasm32"))]
            if matches!(game_mode, GameMode::MainMenu | GameMode::Galaxy) {
                audio_engine.play_music_for_context(
                    if game_mode == GameMode::MainMenu {
                        rebellion_render::MusicContext::MainMenu
                    } else {
                        rebellion_render::MusicContext::GalaxyMap
                    },
                    &sounds_dir,
                    &audio_vol,
                );
            }
            #[cfg(target_arch = "wasm32")]
            if let (Some(engine), Some(bytes)) =
                (browser_menu_audio.as_mut(), browser_main_theme.as_deref())
            {
                engine.load_music_bytes(rebellion_render::MusicTrack::MainTheme, bytes);
            }
            tactical_music_active = false;
        }
        if screen_music != screen_music_playing {
            if let Some(track) = screen_music {
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
            } else if matches!(game_mode, GameMode::MainMenu | GameMode::Galaxy) {
                // Leaving a battle window for the galaxy or the menu
                // restores their score.
                #[cfg(not(target_arch = "wasm32"))]
                audio_engine.play_music_for_context(
                    if game_mode == GameMode::MainMenu {
                        rebellion_render::MusicContext::MainMenu
                    } else {
                        rebellion_render::MusicContext::GalaxyMap
                    },
                    &sounds_dir,
                    &audio_vol,
                );
                #[cfg(target_arch = "wasm32")]
                if let (Some(engine), Some(bytes)) =
                    (browser_menu_audio.as_mut(), browser_main_theme.as_deref())
                {
                    engine.load_music_bytes(rebellion_render::MusicTrack::MainTheme, bytes);
                }
            }
            screen_music_playing = screen_music;
        }

        if tactical_audio_context {
            for cue in tactical_state.take_pending_audio_cues() {
                #[cfg(not(target_arch = "wasm32"))]
                let loaded = audio_engine.play_tactical_sfx(cue.resource_id, &audio_vol);
                #[cfg(target_arch = "wasm32")]
                let loaded = browser_menu_audio
                    .as_mut()
                    .is_some_and(|engine| engine.play_tactical_sfx(cue.resource_id, &audio_vol));
                macroquad::logging::info!(
                    "[audio] context=combat event=0x{:02x} wave={} routed=true loaded={} muted={}",
                    cue.event.event_id(),
                    cue.resource_id,
                    loaded,
                    audio_vol.muted
                );
            }
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

        #[cfg(target_arch = "wasm32")]
        if browser_menu_audio_requested {
            if let Some(engine) = browser_menu_audio.as_mut() {
                engine.try_play_loaded_music(&audio_vol);
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
                );
            }
        }
        #[cfg(all(target_arch = "wasm32", feature = "interface-test-fixtures"))]
        if let Some(request) = tactical_fixture_request {
            if game_mode == GameMode::TacticalCombat && !interface_fixture_emitted {
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

        next_frame().await;
    }
}

/// What a fleet move needs to read when it lands.
struct FleetMoveContext<'a> {
    world: &'a GameWorld,
    movement: &'a MovementState,
    blockaded: &'a HashSet<SystemKey>,
    faction: MissionFaction,
    tick: u64,
}

/// A team of fleets' Move (`0x201`) or Confirmed Move (`0x202`) with its
/// target set: `FUN_00486fb0` validates it (`FUN_00487740`; the move order's
/// validator `FUN_0053c1a0` runs every member's, so one refusal refuses all),
/// then asks first when `FUN_00487cc0` says so, listing one line per member
/// (`FUN_0053c2e0`), or departs. A move onto a fleet joins it
/// (`FUN_004ffc90`, `FUN_004feca0`).
fn issue_fleet_move(
    context: &FleetMoveContext<'_>,
    fleets: &[FleetKey],
    confirmed: bool,
    target: ReleaseTarget,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
    panel_actions: &mut Vec<PanelAction>,
) {
    let world = context.world;
    let is_alliance = context.faction == MissionFaction::Alliance;
    let (destination, join) = match target {
        ReleaseTarget::System(system) => (system, None),
        ReleaseTarget::Fleet { fleet, system } => (system, Some(fleet)),
    };
    let checked = match join {
        Some(target) if !fleets.is_empty() => fleets.iter().try_for_each(|fleet| {
            rebellion_core::fleet_join::validate_join(
                world,
                context.movement,
                &FleetMover::Fleet(*fleet),
                target,
                is_alliance,
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
        }),
        _ => validate_fleets_dispatch(context.movement, world, fleets, destination, is_alliance)
            .map_err(|error| error.to_string()),
    };
    if let Err(error) = checked {
        // port: FUN_00487c90 plays the side's advisor reaction; this line
        // stands in.
        msg_log.push(GameMessage::new(
            context.tick,
            format!("Fleet move rejected: {error}"),
            MessageCategory::Event,
        ));
        return;
    }
    let asks = fleets.iter().any(|fleet| match join {
        Some(target) => rebellion_core::fleet_join::join_confirms(
            world,
            context.blockaded,
            *fleet,
            target,
            confirmed,
        ),
        None => fleet_move_confirms(world, context.blockaded, *fleet, confirmed),
    });
    if !asks {
        panel_actions.extend(fleets.iter().map(|fleet| match join {
            Some(target) => PanelAction::JoinFleet {
                mover: FleetMover::Fleet(*fleet),
                target,
            },
            None => PanelAction::DispatchFleet {
                fleet: *fleet,
                destination,
                troops: Vec::new(),
            },
        }));
        return;
    }
    // FUN_0048a340 → FUN_0049a350: a line per member, its name and days.
    let lines = fleets
        .iter()
        .filter_map(|fleet| {
            let value = world.fleets.get(*fleet)?;
            let days = if value.location == destination {
                0
            } else {
                fleet_transit_ticks(value, world, value.location, destination)?
            };
            Some((fleet_label(world, *fleet)?, days))
        })
        .collect();
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction: context.faction,
        fleets: fleets.to_vec(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        missions: Vec::new(),
        blockaded: false,
        destination,
        join,
        lines,
    });
}

/// A character's Confirmed Move (`0x202` -> `0x241`): the order validates
/// (`FUN_004f4b60`, `1`/`0x28`), then `FUN_00487cc0` opens the confirmation
/// with the trip's days (`FUN_004f4850`). `FUN_0049a350` shows the
/// evacuation warning for a character whose container is a blockaded
/// system.
#[allow(clippy::too_many_arguments)]
fn confirm_character_move(
    world: &GameWorld,
    faction: MissionFaction,
    character: rebellion_core::ids::CharacterKey,
    system: rebellion_core::ids::SystemKey,
    fleet: Option<rebellion_core::ids::FleetKey>,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let days = rebellion_core::missions::move_days(world, character, system);
    let (Some(c), Some(days), true) = (
        world.characters.get(character),
        days,
        rebellion_core::missions::move_place_allowed(world, faction, system, fleet),
    ) else {
        macroquad::logging::info!("[interface] character_move confirm refused");
        msg_log.push(GameMessage::new(
            tick,
            "Move refused".to_string(),
            MessageCategory::Event,
        ));
        return;
    };
    let blockaded = c.current_fleet.is_none()
        && c.current_system
            .and_then(|key| world.systems.get(key))
            .is_some_and(|sys| {
                rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, sys)
            });
    macroquad::logging::info!(
        "[interface] character_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: vec![character],
        special_forces: Vec::new(),
        missions: Vec::new(),
        blockaded,
        destination: system,
        join: fleet,
        lines: vec![(c.name.clone(), days)],
    });
}

/// The missions a mission icon's Abort (`0x250`) acts on, when every one
/// passes `FUN_00521600` (`MissionState::abort_allowed`); empty otherwise
/// or for any other selection.
fn abort_missions(
    world: &GameWorld,
    fog: &rebellion_core::fog::FogState,
    missions: &MissionState,
    selection: Option<MenuObject>,
    faction: MissionFaction,
) -> Vec<u64> {
    let Some(MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Missions,
    }) = selection
    else {
        return Vec::new();
    };
    let ids = rebellion_render::quadrant_icons::own_visible_missions(
        world,
        fog,
        missions,
        Faction::from(faction),
        system,
    );
    if ids.iter().all(|&id| missions.abort_allowed(id, faction)) {
        ids
    } else {
        Vec::new()
    }
}

/// A special force's Confirmed Move (`0x202`), as a character's: the Move's
/// check, then the confirmation with the unit's class name and the trip's
/// days (`FUN_0049a350`, `FUN_004f4850`).
fn confirm_special_force_move(
    world: &GameWorld,
    faction: MissionFaction,
    unit: rebellion_core::ids::SpecialForceKey,
    system: rebellion_core::ids::SystemKey,
    fleet: Option<rebellion_core::ids::FleetKey>,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let member = MissionMember::SpecialForce(unit);
    let days = rebellion_core::missions::member_move_days(world, member, system);
    let (Some(value), Some(days), true) = (
        world.special_forces.get(unit),
        days,
        rebellion_core::missions::move_place_allowed(world, faction, system, fleet),
    ) else {
        macroquad::logging::info!("[interface] special_force_move confirm refused");
        msg_log.push(GameMessage::new(
            tick,
            "Move refused".to_string(),
            MessageCategory::Event,
        ));
        return;
    };
    // The first member is not a fleet; its container is its system, or a
    // fleet, which takes no warning.
    let blockaded = world.systems.values().any(|sys| {
        sys.special_forces.contains(&unit)
            && rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, sys)
    });
    let name = world
        .buildable_classes
        .get(&value.class_dat_id)
        .map_or_else(String::new, |class| class.name.clone());
    macroquad::logging::info!(
        "[interface] special_force_move confirm days={days} blockaded={blockaded}"
    );
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: vec![unit],
        missions: Vec::new(),
        blockaded,
        destination: system,
        join: fleet,
        lines: vec![(name, days)],
    });
}

/// Whether each order of `selection`'s menu passes its own rule (`+0x18`).
fn order_gates(
    selection: Option<MenuObject>,
    world: &GameWorld,
    mission_state: &MissionState,
    movement_state: &MovementState,
    troop_transport_state: &TroopTransportState,
    manufacturing: &ManufacturingState,
    research: &rebellion_core::research::ResearchState,
    player_faction: MissionFaction,
) -> OrderGates {
    let is_alliance = player_faction == MissionFaction::Alliance;
    let side = if is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    // port: a band's manager is its system's holder's; the orders' own
    // +0x18 rules (FUN_0052ae30's class) are untraced.
    let holds = |system| {
        world
            .systems
            .get(system)
            .is_some_and(|value| value.control.is_controlled_by(side))
    };
    OrderGates {
        mission: selection
            .and_then(MenuObject::mission_member)
            .is_some_and(|member| {
                mission_state.mission_order_enabled(world, player_faction, &[member])
            }),
        fleet_move: match selection {
            Some(MenuObject::Fleet(fleet)) => {
                fleet_move_enabled(movement_state, world, fleet, is_alliance)
            }
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Fleets,
            }) => fleets_move_enabled(
                movement_state,
                world,
                &system_side_fleets(world, system, is_alliance),
                is_alliance,
            ),
            _ => false,
        },
        // FUN_0053c4b0 resolves the character; FUN_004f4a00 only builds
        // the command. The order reads the character's side
        // (FUN_00504e60), a prisoner's being its captor's.
        character_move: match selection {
            Some(MenuObject::Character(character)) => {
                world.characters.get(character).is_some_and(|c| {
                    !c.is_killed && rebellion_core::missions::character_side(c) == Some(side)
                })
            }
            _ => false,
        },
        // FUN_00536bc0's Move on a unit, as a character's: the unit is the
        // player's.
        special_force_move: match selection {
            Some(MenuObject::SpecialForce(unit)) => world
                .special_forces
                .get(unit)
                .is_some_and(|value| value.is_alliance == is_alliance),
            _ => false,
        },
        troop_move: match selection {
            Some(MenuObject::Troop(troop)) => {
                troop_transport_state.regiment_move_enabled(world, troop, is_alliance)
            }
            _ => false,
        },
        squadron_move: match selection {
            Some(MenuObject::Fighter { fleet, index }) => world
                .fleets
                .get(fleet)
                .and_then(|value| value.fighters.get(index))
                .is_some_and(|entry| {
                    rebellion_core::squadrons::move_enabled(
                        world,
                        is_alliance,
                        entry.class,
                        SquadronPlace::Fleet(fleet),
                    )
                }),
            Some(MenuObject::PlanetFighter { system, class }) => {
                rebellion_core::squadrons::move_enabled(
                    world,
                    is_alliance,
                    class,
                    SquadronPlace::Planet(system),
                )
            }
            _ => false,
        },
        // FUN_004f9860: the ship's fleet is the player's and in orbit.
        ship_move: match selection {
            Some(MenuObject::Ship { fleet, index, .. }) => {
                fleet_move_enabled(movement_state, world, fleet, is_alliance)
                    && world
                        .fleets
                        .get(fleet)
                        .and_then(|value| value.capital_ships.get(index))
                        .is_some_and(|ship| ship.alive)
            }
            _ => false,
        },
        // port: a system's production areas are its manufacturing
        // facilities' (the 0xa0 managers are not modelled), the player's.
        destination: match selection {
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::System,
            }) => world.systems.get(system).is_some_and(|value| {
                value.manufacturing_facilities.iter().any(|key| {
                    world
                        .manufacturing_facilities
                        .get(*key)
                        .is_some_and(|facility| {
                            facility.side == rebellion_core::dat::Faction::of_alliance(is_alliance)
                        })
                })
            }),
            Some(MenuObject::Producer { system, .. }) => holds(system),
            _ => false,
        },
        // Build opens Build Selection on what the band's manager lists
        // (FUN_0052e580).
        build: match selection {
            Some(MenuObject::Producer { system, area }) => {
                holds(system)
                    && !rebellion_core::build_selection::listed_classes(
                        world,
                        research,
                        area,
                        is_alliance,
                    )
                    .is_empty()
            }
            _ => false,
        },
        // Stop clears a band that is building (manual p. 84).
        stop: match selection {
            Some(MenuObject::Producer { system, area }) => {
                holds(system)
                    && manufacturing
                        .queue(system, area)
                        .is_some_and(|queue| !queue.is_empty())
            }
            _ => false,
        },
        // port: the player names its own fleets and ships; FUN_004f6e60
        // refuses a destroyed object (FUN_0053a000). The order's own +0x18
        // rule is untraced.
        rename: match selection {
            Some(MenuObject::Fleet(fleet)) => world
                .fleets
                .get(fleet)
                .is_some_and(|value| value.is_alliance == is_alliance),
            Some(MenuObject::Ship { fleet, index, .. }) => {
                world.fleets.get(fleet).is_some_and(|value| {
                    value.is_alliance == is_alliance
                        && value
                            .capital_ships
                            .get(index)
                            .is_some_and(|ship| ship.alive)
                })
            }
            _ => false,
        },
        bombard: bombard_team(world, selection, is_alliance).map_or(
            [false; 4],
            |(team, system)| {
                [0x220, 0x221, 0x222, 0x223].map(|kind| {
                    rebellion_core::bombardment::order_enabled(
                        world,
                        movement_state,
                        is_alliance,
                        &team,
                        system,
                        kind,
                    )
                })
            },
        ),
        assault: bombard_team(world, selection, is_alliance).is_some_and(|(team, system)| {
            rebellion_core::assault::order_enabled(
                world,
                movement_state,
                troop_transport_state,
                is_alliance,
                &team,
                system,
            )
        }),
        // FUN_004ed750 for each of 0x260..0x263.
        command: command_rows(world, mission_state, selection, is_alliance),
        // FUN_004ed460 (the Retire order's +0x18 through event 0x230).
        // Set where a mission icon is chosen, with the player's fog
        // (`abort_gate`).
        abort: false,
        retire: selection
            .and_then(MenuObject::mission_member)
            .and_then(|member| retire_check(world, mission_state, member, is_alliance))
            .is_some_and(|result| result.is_ok()),
    }
}

/// The Command submenu's rows on `selection`: which of `0x260..0x263`
/// `FUN_004ed750` allows. None is checked (`object_menu.rs`,
/// `ObjectMenuRow::entry`).
fn command_rows(
    world: &GameWorld,
    missions: &MissionState,
    selection: Option<MenuObject>,
    is_alliance: bool,
) -> [bool; 4] {
    use rebellion_core::character_command::{order_allowed, CommandKind};
    let Some(MenuObject::Character(key)) = selection else {
        return [false; 4];
    };
    let Some(c) = world.characters.get(key) else {
        return [false; 4];
    };
    let en_route = missions.is_en_route(rebellion_core::missions::MissionMember::Character(key));
    [0x260, 0x261, 0x262, 0x263].map(|order| {
        let kind = CommandKind::of_order(order).unwrap_or(CommandKind::None);
        order_allowed(c, kind, is_alliance, en_route).is_ok()
    })
}

/// The Retire order's check on `member` (`FUN_004ed460` for a character,
/// `FUN_00533ba0` for a special force), with the name its confirmation
/// lists: a character's own, a special force's class name. `None` when the
/// member is gone.
fn retire_check(
    world: &GameWorld,
    missions: &MissionState,
    member: MissionMember,
    is_alliance: bool,
) -> Option<Result<String, rebellion_core::character_retire::Refusal>> {
    use rebellion_core::character_retire::{order_allowed, special_force_allowed};
    let en_route = missions.is_en_route(member);
    match member {
        MissionMember::Character(key) => {
            let c = world.characters.get(key)?;
            Some(order_allowed(c, is_alliance, en_route).map(|()| c.name.clone()))
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let name = world
                .buildable_classes
                .get(&unit.class_dat_id)
                .map_or_else(String::new, |class| class.name.clone());
            Some(special_force_allowed(unit, is_alliance, en_route).map(|()| name))
        }
    }
}

/// The team and system of a Planetary Bombardment order on `selection`: a
/// fleet at its system, or a fleet icon's system with its fleets of the
/// player's side (`FUN_00512700`, kind `0x10`).
fn bombard_team(
    world: &GameWorld,
    selection: Option<MenuObject>,
    is_alliance: bool,
) -> Option<(Vec<FleetKey>, SystemKey)> {
    match selection? {
        MenuObject::Fleet(fleet) => Some((vec![fleet], world.fleets.get(fleet)?.location)),
        MenuObject::SystemIcon {
            system,
            quadrant: Quadrant::Fleets,
        } => Some((
            rebellion_core::movement::system_side_fleets(world, system, is_alliance),
            system,
        )),
        _ => None,
    }
}

/// Carry out an Agent menu command or its accelerator: Manage Garrisons and
/// Manage Production toggle their module (`FUN_00439d60`), Agent Advice its
/// bit (`FUN_00439e80`).
/// Where a Finder's choice opens through `FUN_00429440`.
#[derive(Debug, Clone, Copy)]
enum FinderTarget {
    /// A system alone: its sector window.
    Sector,
    /// The System Defenses window (kind 10).
    Defenses,
    /// The Fleet window (kind 4) with the fleet selected.
    Fleet(rebellion_core::ids::FleetKey),
    /// The Missions window (kind 11) showing the member's mission.
    Missions(rebellion_core::missions::MissionMember),
}

/// Carry out what the Message Index asks: the Advice speed hold
/// (`FUN_00487ff0`), marking read (`FUN_0048a530`, `FUN_00469de0`) and
/// deleting (`FUN_005f54a0`).
fn apply_message_index_actions(
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
            MessageIndexAction::MessageDisplayed(id) => log.mark_message_read(id),
            MessageIndexAction::Close => {}
        }
    }
}

fn apply_agent_command(
    agent: &mut PlayerAgent,
    command: rebellion_render::agent_menu::AgentCommand,
    source: &str,
) {
    use rebellion_render::agent_menu::AgentCommand;
    let (name, on) = match command {
        AgentCommand::ManageGarrisons => (
            "manage_garrisons",
            agent.toggle(AutomationModule::Garrisons),
        ),
        AgentCommand::ManageProduction => (
            "manage_production",
            agent.toggle(AutomationModule::Production),
        ),
        AgentCommand::AgentAdvice => ("agent_advice", agent.toggle_advice()),
    };
    macroquad::logging::info!(
        "[agent] command={name} status={} source={source}",
        if on { "on" } else { "off" }
    );
}

// ---------------------------------------------------------------------------
// Panel action handler
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and UI inputs at this existing integration boundary."
)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn apply_panel_action(
    action: PanelAction,
    world: &mut GameWorld,
    map_state: &mut GalaxyMapState,
    mfg_state: &mut ManufacturingState,
    mission_state: &mut MissionState,
    movement_state: &mut MovementState,
    troop_transport_state: &mut TroopTransportState,
    fog_state: &mut FogState,
    ai_state: &mut AIState,
    msg_log: &mut MessageLog,
    player_faction: &mut MissionFaction,
    clock: &mut GameClock,
    dual_ai_mode: &mut bool,
    secondary_ai_state: &mut Option<AIState>,
    victory_state: &mut VictoryState,
    campaign_config: CampaignConfig,
    blockade_state: &mut BlockadeState,
    event_state: &EventState,
    mod_runtime: &mut rebellion_data::mods::ModRuntime,
    message_texts: &std::collections::HashMap<u16, String>,
    assault_summaries: &mut rebellion_render::assault_summary::AssaultSummaryState,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
    #[cfg(not(target_arch = "wasm32"))] audio_vol: &AudioVolumeState,
    #[cfg(not(target_arch = "wasm32"))] _sounds_dir: &Path,
) {
    match action {
        // FUN_00578f30 → FUN_00556390 (ghidra/notes/regiment-unload.md).
        PanelAction::MoveRegiment { troop, target } => {
            let expected_is_alliance = *player_faction == MissionFaction::Alliance;
            let result = if world
                .troops
                .get(troop)
                .is_some_and(|value| value.is_alliance == expected_is_alliance)
            {
                troop_transport_state.move_regiment(
                    world,
                    movement_state,
                    blockade_state.blockaded_systems(),
                    troop,
                    target,
                    clock.tick,
                )
            } else {
                Err(rebellion_core::troop_transport::TroopTransportError::WrongFaction)
            };
            // port: FUN_00487c90's advisor reaction; this line stands in.
            if let Err(error) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Regiment move rejected: {error}"),
                    MessageCategory::Event,
                ));
            }
        }
        // FUN_00557ce0's 0x201 for a squadron (rebellion_core::squadrons).
        PanelAction::MoveSquadron { class, from, to } => {
            let alliance = *player_faction == MissionFaction::Alliance;
            // port: FUN_00487c90's advisor reaction; this line stands in.
            if let Err(error) = troop_transport_state
                .squadrons
                .move_squadrons(world, alliance, class, 1, from, to, clock.tick)
            {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Squadron move rejected: {error}"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::FocusFleetSystem(sys_key) => {
            if world.systems.contains_key(sys_key) {
                map_state.selected_system = Some(sys_key);
            }
        }
        // ghidra/notes/fleet-join-split.md.
        PanelAction::JoinFleet { mover, target } => {
            let result = rebellion_core::fleet_join::join_fleet(
                world,
                movement_state,
                troop_transport_state,
                &mover,
                target,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Fleet move rejected: {error}"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::SetDestination {
            system,
            area,
            destination,
        } => {
            let areas = area.map_or(
                rebellion_core::manufacturing::ProductionArea::ALL.to_vec(),
                |area| vec![area],
            );
            for area in &areas {
                mfg_state.set_destination(system, *area, destination);
            }
            macroquad::logging::info!(
                "[interface] command=0x214 destination=production_destination status=set areas={} to={}",
                areas.len(),
                world
                    .systems
                    .get(destination)
                    .map_or("", |value| value.name.as_str())
            );
        }
        PanelAction::Rename { fleet, ship, name } => {
            let renamed = match ship {
                Some(index) => world.rename_ship(fleet, index, &name),
                None => world.rename_fleet(fleet, &name),
            };
            macroquad::logging::info!(
                "[interface] command=0x203 destination=rename status={} name={name}",
                if renamed { "applied" } else { "refused" }
            );
        }
        PanelAction::CreateFleet {
            fleet,
            ships,
            roster,
        } => {
            let result = rebellion_core::fleet_join::create_fleet(
                world,
                movement_state,
                troop_transport_state,
                fleet,
                &ships,
                roster,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Create Fleet rejected: {error}"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::MoveShips {
            fleet,
            ships,
            roster,
            system,
        } => {
            let result = rebellion_core::fleet_join::move_ships_to_system(
                world,
                movement_state,
                troop_transport_state,
                fleet,
                &ships,
                roster,
                system,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Fleet move rejected: {error}"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::DispatchFleet {
            fleet,
            destination,
            troops,
        } => {
            let expected_is_alliance = *player_faction == MissionFaction::Alliance;
            if let Err(error) = validate_fleet_dispatch(
                movement_state,
                world,
                fleet,
                destination,
                expected_is_alliance,
            ) {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Fleet move rejected: {error}"),
                    MessageCategory::Event,
                ));
                return;
            }
            if let Err(error) = troop_transport_state.embark(world, fleet, &troops) {
                if !troops.is_empty() {
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("Troop embarkation rejected: {error}"),
                        MessageCategory::Event,
                    ));
                    return;
                }
            }
            match begin_faction_fleet_transit(
                movement_state,
                world,
                fleet,
                destination,
                expected_is_alliance,
            ) {
                Ok(departure) => {
                    let origin_name = world
                        .systems
                        .get(departure.origin)
                        .map_or("Unknown", |system| system.name.as_str());
                    let destination_name = world
                        .systems
                        .get(departure.destination)
                        .map_or("Unknown", |system| system.name.as_str());
                    msg_log.push(GameMessage::at_system(
                        clock.tick,
                        format!(
                            "Fleet departed {} for {} ({} days)",
                            origin_name, destination_name, departure.transit_ticks,
                        ),
                        MessageCategory::Event,
                        departure.destination,
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        audio_engine.play_sfx(SfxKind::FleetDeparture, audio_vol);
                    }
                }
                Err(error) => {
                    if !troops.is_empty() {
                        let origin = world
                            .fleets
                            .get(fleet)
                            .map(|value| value.location)
                            .unwrap_or_default();
                        let _ =
                            troop_transport_state.disembark_selected(world, fleet, origin, &troops);
                    }
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("Fleet move rejected: {error}"),
                        MessageCategory::Event,
                    ));
                }
            }
        }
        PanelAction::StopProduction { system, area } => {
            mfg_state.stop(system, area);
            macroquad::logging::info!(
                "[interface] command=0x213 destination=production_stop status=stopped area={area:?} system={}",
                world.systems.get(system).map_or("", |value| value.name.as_str())
            );
        }
        // FUN_00438980 -> FUN_0041ce20: the units replace what the band
        // was building, each at its class cost in build points.
        PanelAction::BuildProduction {
            system,
            area,
            kind,
            count,
        } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if let Some(items) = rebellion_core::build_selection::order_items(
                world,
                system,
                area,
                kind,
                count,
                is_alliance,
            ) {
                mfg_state.build_units(system, items);
                macroquad::logging::info!(
                    "[interface] command=0x21x destination=production_build status=queued area={area:?} count={count} system={}",
                    world.systems.get(system).map_or("", |value| value.name.as_str())
                );
            }
        }
        PanelAction::DispatchMission {
            kind,
            faction,
            team,
            decoys,
            target,
            target_character,
            target_object,
            tick,
        } => {
            // The first agent names the order in its messages.
            let char_name = team
                .iter()
                .find_map(|member| member.character())
                .and_then(|key| world.characters.get(key))
                .map_or_else(|| "Your team".into(), |c| c.name.clone());
            let request = rebellion_core::missions::MissionRequest {
                kind,
                faction,
                team,
                decoys,
                target_system: target,
                target_character,
                target_object,
                tick,
            };
            if let Err(refusal) = mission_state.dispatch_guarded(request, world) {
                // port: FUN_00487c90 shows no text; the side's advisor
                // schedules a reaction (`mission-dialog.md`, "Refusal"). These
                // lines stand in until the advisor plays it (audit P34).
                let text = match refusal {
                    rebellion_core::missions::MissionRefusal::EmptyTeam => {
                        format!("{char_name} is a prisoner and cannot lead a mission")
                    }
                    rebellion_core::missions::MissionRefusal::MemberUnavailable(_) => {
                        format!("{char_name} cannot join this mission")
                    }
                    rebellion_core::missions::MissionRefusal::TargetUnavailable => {
                        "This target cannot be sabotaged".to_string()
                    }
                    rebellion_core::missions::MissionRefusal::MembersNotAllowed => {
                        format!("{char_name} cannot undertake this mission")
                    }
                };
                msg_log.push(GameMessage::new(clock.tick, text, MessageCategory::Mission));
                return;
            }
            let sys_name = world
                .systems
                .get(target)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            let kind_name = match kind {
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
                MissionKind::Adrift => "Adrift",
                MissionKind::JediTraining => "Jedi Training",
                MissionKind::Dagobah => "Dagobah",
                MissionKind::Bounty => "Bounty",
                MissionKind::Palace => "Palace",
                MissionKind::Pickup => "Pickup",
            };
            msg_log.push(GameMessage::at_system(
                clock.tick,
                format!("{char_name} dispatched on {kind_name} mission to {sys_name}"),
                MessageCategory::Mission,
                target,
            ));
        }
        // Save/load actions are handled by the caller before dispatching here;
        // they require access to the full save state and are not routed through
        // this helper.
        PanelAction::SelectFaction(_)
        | PanelAction::OpenSaveLoad
        | PanelAction::SaveGame { .. }
        | PanelAction::LoadGame { .. }
        | PanelAction::DeleteSave { .. }
        | PanelAction::CloseSaveLoadPanel
        | PanelAction::OpenModManager => {
            // Handled by UI state toggle (not a world mutation)
        }
        PanelAction::ToggleMod { ref name } => {
            mod_runtime.toggle_mod(name);
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Toggled mod: {name}"),
                MessageCategory::Event,
            ));
        }
        PanelAction::ReloadMods => {
            mod_runtime.refresh();
            let mod_errors = mod_runtime.apply_enabled(world);
            for err in &mod_errors {
                eprintln!("Mod reload error: {err:?}");
            }
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Reloaded {} mods", mod_runtime.discovered.len()),
                MessageCategory::Event,
            ));
        }
        // FUN_00486fb0 -> FUN_00487740: the order validates (+0x1c), then
        // executes, or FUN_00487c90 reports the refusal. port: the draws
        // come from the day.
        PanelAction::OrderBombardment {
            fleets,
            system,
            kind,
        } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if rebellion_core::bombardment::order_enabled(
                world,
                movement_state,
                is_alliance,
                &fleets,
                system,
                kind,
            ) {
                let mut bombard = BombardEnv::new(clock.tick, Some(victory_state.alliance_hq));
                let outcome = bombard
                    .bombard_reported(
                        world,
                        troop_transport_state,
                        Faction::of_alliance(is_alliance),
                        &fleets,
                        system,
                        kind,
                    )
                    .clone();
                for (system, losses) in &bombard.losses {
                    blockade_state.record_losses(*system, *losses);
                }
                log_bombardment(world, system, &outcome, clock.tick, msg_log);
                present_assault_reports(
                    std::mem::take(&mut bombard.reports),
                    world,
                    *player_faction,
                    message_texts,
                    assault_summaries,
                    msg_log,
                    clock.tick,
                );
                if outcome.system_destroyed {
                    destroy_planet(
                        world,
                        system,
                        &outcome.killed,
                        mfg_state,
                        blockade_state,
                        msg_log,
                        clock.tick,
                        message_texts,
                    );
                }
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Planetary bombardment refused".to_string(),
                    MessageCategory::Combat,
                ));
            }
        }
        // FUN_00486fb0 -> FUN_00487740: the Move order's 0x241 validates
        // (FUN_004f4b60: a target of another side is 1/0x28), then its
        // command 0x250 creates the Move mission (FUN_0054d360), or
        // FUN_00487c90 reports the refusal.
        PanelAction::MoveCharacter {
            character,
            system,
            fleet,
        } => {
            let result = if rebellion_core::missions::move_place_allowed(
                world,
                *player_faction,
                system,
                fleet,
            ) {
                mission_state
                    .dispatch_guarded(
                        rebellion_core::missions::MissionRequest::single(
                            MissionKind::Move,
                            *player_faction,
                            character,
                            system,
                            None,
                            clock.tick,
                        ),
                        world,
                    )
                    .inspect(|&id| {
                        if let Some(fleet) = fleet {
                            mission_state.set_target_fleet(id, fleet);
                        }
                    })
                    .map_err(|refusal| format!("{refusal:?}"))
            } else {
                Err("1/0x28".to_string())
            };
            macroquad::logging::info!(
                "[interface] character_move character={:?} to={:?} fleet={:?} result={:?}",
                world.characters.get(character).map(|c| c.name.as_str()),
                world.systems.get(system).map(|s| s.name.as_str()),
                fleet.and_then(|key| fleet_label(world, key)),
                result,
            );
            if result.is_err() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Move refused".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        // A special force's Move: the Move mission 0x41000001 with the unit
        // as its team, as a character's.
        PanelAction::MoveSpecialForce { unit, system, fleet } => {
            let result = if rebellion_core::missions::move_place_allowed(
                world,
                *player_faction,
                system,
                fleet,
            ) {
                mission_state
                    .dispatch_guarded(
                        rebellion_core::missions::MissionRequest {
                            kind: MissionKind::Move,
                            faction: *player_faction,
                            team: vec![MissionMember::SpecialForce(unit)],
                            decoys: Vec::new(),
                            target_system: system,
                            target_character: None,
                            target_object: None,
                            tick: clock.tick,
                        },
                        world,
                    )
                    .inspect(|&id| {
                        if let Some(fleet) = fleet {
                            mission_state.set_target_fleet(id, fleet);
                        }
                    })
                    .map_err(|refusal| format!("{refusal:?}"))
            } else {
                Err("1/0x28".to_string())
            };
            macroquad::logging::info!(
                "[interface] special_force_move unit={unit:?} to={:?} fleet={:?} result={result:?}",
                world.systems.get(system).map(|s| s.name.as_str()),
                fleet.and_then(|key| fleet_label(world, key)),
            );
            if result.is_err() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Move refused".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        // FUN_00486fb0 -> FUN_00487740: the Command order validates
        // (FUN_004ed750), then takes effect (FUN_00561bb0), or FUN_00487c90
        // reports the refusal.
        PanelAction::OrderCommand { character, order } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            let Some(kind) = rebellion_core::character_command::CommandKind::of_order(order)
            else {
                return;
            };
            let en_route = mission_state
                .is_en_route(rebellion_core::missions::MissionMember::Character(character));
            let result = rebellion_core::character_command::give(
                world,
                character,
                kind,
                is_alliance,
                en_route,
            );
            macroquad::logging::info!(
                "[interface] command={order:#x} character={:?} result={:?} now={:?}",
                world.characters.get(character).map(|c| c.name.as_str()),
                result.as_ref().map(Vec::len),
                world.characters.get(character).map(|c| c.command),
            );
            if result.is_err() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Command refused".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        // FUN_0041ce20(order, 1): the confirmed Retire validates again
        // (FUN_004ed460) and fires event 0x230 (FUN_00540f40 ->
        // FUN_00534c00), or FUN_00487c90 reports the refusal.
        PanelAction::Retire(member) => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            let en_route = mission_state.is_en_route(member);
            let result = match member {
                MissionMember::Character(key) => {
                    rebellion_core::character_retire::retire(world, key, is_alliance, en_route)
                }
                MissionMember::SpecialForce(key) => {
                    rebellion_core::character_retire::retire_special_force(
                        world,
                        key,
                        is_alliance,
                        en_route,
                    )
                }
            };
            macroquad::logging::info!(
                "[interface] command=0x242 destination=retire member={member:?} result={result:?}"
            );
            if result.is_err() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Retire refused".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        // FUN_00486fb0 -> FUN_00487740 -> FUN_00563390: the order
        // validates (+0x1c), then executes (FUN_0058c120), or FUN_00487c90
        // reports the refusal. port: the draws come from the day.
        PanelAction::OrderAssault { fleets, system } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if rebellion_core::assault::order_enabled(
                world,
                movement_state,
                troop_transport_state,
                is_alliance,
                &fleets,
                system,
            ) {
                let mut env = BombardEnv::new(clock.tick, Some(victory_state.alliance_hq));
                let outcome = env
                    .assault(
                        world,
                        troop_transport_state,
                        Faction::of_alliance(is_alliance),
                        &fleets,
                        system,
                    )
                    .clone();
                for (system, losses) in &env.losses {
                    blockade_state.record_losses(*system, *losses);
                }
                log_assault(world, system, &outcome, clock.tick, msg_log);
                present_assault_reports(
                    std::mem::take(&mut env.reports),
                    world,
                    *player_faction,
                    message_texts,
                    assault_summaries,
                    msg_log,
                    clock.tick,
                );
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Planetary assault refused".to_string(),
                    MessageCategory::Combat,
                ));
            }
        }
        PanelAction::AdvanceTicks(n) => {
            // The clock owes N days; the frame loop runs each through the
            // same simulation step as a day of normal play.
            clock.queue_days(n);
        }
        // The caller routes speed through the clock's pause stop.
        PanelAction::SetGameSpeed(_) => {}
        PanelAction::ToggleDualAI => {
            *dual_ai_mode = !*dual_ai_mode;
            if *dual_ai_mode {
                // Create persistent second AI for the opposite faction
                let second_faction = match ai_state.faction {
                    Some(AiFaction::Empire) => AiFaction::Alliance,
                    _ => AiFaction::Empire,
                };
                *secondary_ai_state = Some(AIState::new(second_faction));
            } else {
                *secondary_ai_state = None;
            }
            let state_str = if *dual_ai_mode { "ENABLED" } else { "DISABLED" };
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Dual AI mode {state_str}"),
                MessageCategory::Event,
            ));
        }
        PanelAction::ForceVictoryCheck => {
            let tick_ev = rebellion_core::tick::TickEvent { tick: clock.tick };
            let result = rebellion_core::victory::VictorySystem::check(
                victory_state,
                world,
                &[tick_ev],
                campaign_config.victory_conditions,
            );
            if let Some(outcome) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Victory check: {outcome:?}"),
                    MessageCategory::Event,
                ));
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Victory check: no winner yet".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::RevealAllFog => {
            // Reveal all systems in fog state
            for (sys_key, _) in &world.systems {
                fog_state.reveal(sys_key);
            }
        }
        PanelAction::ExportGameLog => {
            // Resolve system keys to names before export
            msg_log.resolve_system_names(|key| world.systems.get(key).map(|s| s.name.clone()));
            let path = std::path::PathBuf::from("game_log.jsonl");
            match msg_log.export_jsonl(&path) {
                Ok(()) => {
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("Exported game log to {}", path.display()),
                        MessageCategory::Event,
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to export game log: {e}");
                }
            }
        }
        PanelAction::ShowGameStats => {
            let alliance_systems = world
                .systems
                .values()
                .filter(|s| s.control.is_controlled_by(Faction::Alliance))
                .count();
            let empire_systems = world
                .systems
                .values()
                .filter(|s| s.control.is_controlled_by(Faction::Empire))
                .count();
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Stats: tick {}, Alliance {} systems, Empire {} systems, {} fleets, {} characters",
                    clock.tick, alliance_systems, empire_systems, world.fleets.len(), world.characters.len()),
                MessageCategory::Event,
            ));
        }
        PanelAction::ListActiveMissions => {
            let missions = mission_state.missions();
            if missions.is_empty() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "No active missions".to_string(),
                    MessageCategory::Mission,
                ));
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("{} active missions:", missions.len()),
                    MessageCategory::Mission,
                ));
                for m in missions {
                    let char_name = m
                        .lead_character()
                        .and_then(|key| world.characters.get(key))
                        .map_or_else(|| "Unknown".into(), |c| c.name.clone());
                    let sys_name = world
                        .systems
                        .get(m.target_system)
                        .map_or_else(|| "unknown".into(), |s| s.name.clone());
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("  {:?} — {} at {}", m.kind, char_name, sys_name),
                        MessageCategory::Mission,
                    ));
                }
            }
        }
        PanelAction::ListActiveFleets => {
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("{} fleets:", world.fleets.len()),
                MessageCategory::Event,
            ));
            for (_, fleet) in &world.fleets {
                let sys_name = world
                    .systems
                    .get(fleet.location)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let faction = if fleet.is_alliance {
                    "Alliance"
                } else {
                    "Empire"
                };
                let ship_count = fleet.ship_count() as usize
                    + fleet
                        .fighters
                        .iter()
                        .map(|e| e.count as usize)
                        .sum::<usize>();
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("  {faction} fleet at {sys_name} — {ship_count} ships"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::ShowEventCount => {
            let total = event_state.events().len();
            let fired = event_state
                .events()
                .iter()
                .filter(|e| event_state.has_fired(e.id))
                .count();
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Events: {total} defined, {fired} fired"),
                MessageCategory::Event,
            ));
        }

    }
}

// ---------------------------------------------------------------------------
// Effect application helpers
// ---------------------------------------------------------------------------

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
fn push_character_health(
    log: &mut MessageLog,
    world: &GameWorld,
    character: rebellion_core::ids::CharacterKey,
    tick: u64,
) {
    let Some(c) = world.characters.get(character) else {
        return;
    };
    let (text, picture) = if c.injury != 0 {
        (
            format!("{} Injured: {} has been injured.", c.name, c.name),
            pictures::injured(),
        )
    } else {
        (
            format!(
                "{} Healed: {} has been healed of all injuries.",
                c.name, c.name
            ),
            pictures::healed((c.dat_id, c.is_major)),
        )
    };
    log.push(filed(
        GameMessage::new(tick, text, MessageCategory::Event).with_picture_if(Some(picture)),
        MessageRail::Mission,
        character_audience(world, character),
    ));
}

/// The Character Injured message (`0x106`) for each injury the mission
/// detection dealt, after [`rebellion_core::missions::apply_advance_effects`]
/// stored it.
fn push_advance_injuries(
    log: &mut MessageLog,
    world: &GameWorld,
    effects: &[MissionEffect],
    tick: u64,
) {
    for effect in effects {
        if let MissionEffect::CharacterInjured { character, injury } = effect {
            if *injury > 0 {
                push_character_health(log, world, *character, tick);
            }
        }
    }
}

/// Notification `0x362` for each Force discovery (timer `0x389`), built as
/// `FUN_0048f1e0` (`story_messages::force_user_discovered`).
fn push_force_discoveries(
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

fn apply_mission_result(
    result: &rebellion_core::missions::MissionResult,
    world: &mut GameWorld,
    research_state: &mut ResearchState,
    log: &mut MessageLog,
    message_texts: &std::collections::HashMap<u16, String>,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
    #[cfg(not(target_arch = "wasm32"))] audio_vol: &AudioVolumeState,
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
        | MissionKind::Adrift
        | MissionKind::JediTraining
        | MissionKind::Dagobah
        | MissionKind::Bounty
        | MissionKind::Palace
        | MissionKind::Pickup => MessageCategory::Mission,
    };
    // FUN_004927c0: the mission type's picture and the leader's portrait.
    let picture = {
        use pictures::ReportedMission;
        pictures::mission_report(
            match result.kind {
                MissionKind::Diplomacy => ReportedMission::Diplomacy,
                MissionKind::Espionage => ReportedMission::Espionage,
                MissionKind::InciteUprising => ReportedMission::InciteUprising,
                MissionKind::SubdueUprising => ReportedMission::SubdueUprising,
                _ => ReportedMission::Other,
            },
            result.outcome == rebellion_core::missions::MissionOutcome::Success,
            pictures::reader(result.faction == MissionFaction::Alliance),
            result
                .character
                .and_then(|character| pictures::portrait_key(world, character)),
        )
    };
    // Notifications 0x16 and 0x17, Mission Report and Mission Failed.
    log.push(filed(
        GameMessage::at_system(
            result.tick,
            format!("{faction_name} {kind_name} mission at {sys_name} {outcome_str}"),
            category,
            result.target_system,
        )
        .with_picture(picture),
        MessageRail::Mission,
        Some(RailAudience::side(
            result.faction == MissionFaction::Alliance,
        )),
    ));

    // SFX + voice lines for mission outcomes
    #[cfg(not(target_arch = "wasm32"))]
    if result.outcome == rebellion_core::missions::MissionOutcome::Success {
        audio_engine.play_sfx(SfxKind::MissionSuccess, audio_vol);
    } else {
        audio_engine.play_sfx(SfxKind::MissionFail, audio_vol);
    }

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
                let tree_name = match tech {
                    rebellion_core::research::TechType::Ship => "Ship",
                    rebellion_core::research::TechType::Troop => "Troop",
                    rebellion_core::research::TechType::Facility => "Facility",
                };
                let was_done = research_state.progress(is_alliance, *tech).done;
                for level in research_state.add_points(world, is_alliance, *tech, *points) {
                    // Notifications 0x125..0x127, Research Complete
                    // (FUN_00499aa0): the class the level opens.
                    let (title, text) = match rebellion_render::story_messages::research_complete_texts(
                        world,
                        is_alliance,
                        *tech,
                        level,
                        message_texts,
                    ) {
                        Some((title, body)) => (Some(title), body),
                        None if message_texts.is_empty() => (
                            None,
                            format!("{faction_name} {tree_name} tech advanced to level {level}"),
                        ),
                        None => continue,
                    };
                    let mut message = GameMessage::new(result.tick, text, MessageCategory::Event)
                        .with_picture_if(Some(pictures::research()));
                    if let Some(title) = title {
                        message = message.with_title(title);
                    }
                    log.push(filed(
                        message,
                        MessageRail::Manufacturing,
                        Some(RailAudience::side(is_alliance)),
                    ));
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
                                .with_picture_if(Some(pictures::research())),
                            MessageRail::Manufacturing,
                            Some(RailAudience::side(is_alliance)),
                        ));
                    }
                }
            }
            MissionEffect::UprisingIncident(event) => {
                rebellion_core::uprising::apply_uprising_event(world, event);
            }
            MissionEffect::SystemIntelligenceGathered { system, .. } => {
                // Reveal fog: mark system as explored (full implementation in fog task).
                if let Some(sys) = world.systems.get_mut(*system) {
                    sys.exploration_status = rebellion_core::dat::ExplorationStatus::Explored;
                }
            }
            MissionEffect::CharacterRecruited {
                character,
                system,
                faction,
                pool_emptied,
            } => rebellion_core::missions::recruit_character(
                world,
                *character,
                *system,
                *faction,
                *pool_emptied,
            ),
            MissionEffect::TargetSabotaged { target, .. } => {
                rebellion_core::missions::destroy_target(world, *target);
                if matches!(
                    target,
                    rebellion_core::missions::MissionTarget::DeathStar(_)
                ) {
                    // Notification 0x23, Death Star Sabotaged.
                    log.push(filed(
                        GameMessage::new(
                            result.tick,
                            "The Death Star has been sabotaged.".to_string(),
                            MessageCategory::Mission,
                        ),
                        MessageRail::Mission,
                        Some(RailAudience::side(
                            result.faction == MissionFaction::Alliance,
                        )),
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
                    push_character_health(log, world, *character, result.tick);
                }
            }
            // The detection's Force gains come with the advance
            // (`apply_advance_effects`); a result applies one the same way.
            MissionEffect::ForceExperienceGained { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*amount);
                }
            }
            // The advance's effects (`apply_advance_effects`).
            MissionEffect::MemberEnteredAbode { member, abode } => {
                rebellion_core::missions::enter_abode(world, *member, *abode);
            }
            MissionEffect::DagobahCompleted {
                character,
                force_gain,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*force_gain);
                    c.dagobah_completed = true;
                }
            }
            // FUN_005712b0 -> FUN_004ee1d0: a Jedi Training gain.
            MissionEffect::ForceTrainingGained { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_training(*amount);
                }
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
                if let Some(c) = world.characters.get_mut(*character) {
                    c.is_alliance = *escaped_to_alliance;
                    c.is_empire = !*escaped_to_alliance;
                    c.is_captive = false;
                    c.captured_by = None;
                    c.capture_tick = None;
                }
                let name = world
                    .characters
                    .get(*character)
                    .map_or_else(|| "Unknown".into(), |c| c.name.clone());
                log.push(
                    GameMessage::new(
                        result.tick,
                        format!("{name} has escaped captivity!"),
                        MessageCategory::Event,
                    )
                    .with_picture(pictures::escaped()),
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
fn battle_winner(
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

fn apply_space_combat_result(
    result: &rebellion_core::combat::SpaceCombatResult,
    world: &mut GameWorld,
) {
    rebellion_data::integrator::apply_space_combat_result_inner(result, world);
}

/// The messages a bombardment leaves: what it destroyed, and the Rebel
/// HQ's destruction (notification `0x1f`).
fn log_bombardment(
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
        log.push(filed(
            GameMessage::new(
                tick,
                format!("Alliance headquarters destroyed at {name}"),
                MessageCategory::Combat,
            ),
            MessageRail::Resource,
            Some(RailAudience::Both),
        ));
    }
}

/// The Assault Summary (`ghidra/notes/planetary-assault.md`): each side's
/// losses and the regiments that landed.
fn log_assault(
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
fn present_assault_reports(
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
        if report.opens_at_once(Faction::of_alliance(alliance)) {
            summaries.open(world, report, message_texts, None);
            continue;
        }
        let (title, body) =
            rebellion_render::assault_summary::summary_texts(&report, world, message_texts);
        let picture = rebellion_render::message_log::MessagePicture {
            background: report.picture(),
            foreground: None,
        };
        log.push(
            GameMessage::at_system(tick, body, MessageCategory::Combat, report.system)
                .with_title(title)
                .with_picture(picture)
                .on_rail(MessageRail::Conflict, RailAudience::side(alliance))
                .with_assault_report(report),
        );
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
fn destroy_planet(
    world: &mut GameWorld,
    system: SystemKey,
    killed: &[rebellion_core::ids::CharacterKey],
    mfg_state: &mut ManufacturingState,
    blockade_state: &mut BlockadeState,
    msg_log: &mut MessageLog,
    tick: u64,
    message_texts: &std::collections::HashMap<u16, String>,
) {
    let name = world
        .systems
        .get(system)
        .map_or_else(|| "Unknown".to_string(), |s| s.name.clone());
    // The side the system had before the blast (FUN_004950d0 +0x64).
    let holder = world.systems.get(system).and_then(|s| s.control.faction());
    if let Some(sys) = world.systems.get_mut(system) {
        sys.is_destroyed = true;
    }
    // Drain the telemetry out-param into the message log so
    // interactive play surfaces the killed characters
    // immediately; the run_simulation_tick flow does the same
    // via the `PerceptionIntegrator`.
    let mut cleanup_effects: Vec<rebellion_core::effects::GameEffect> = Vec::new();
    rebellion_core::death_star::cleanup_destroyed_system(
        world,
        system,
        killed,
        mfg_state,
        blockade_state,
        &mut cleanup_effects,
    );
    for effect in cleanup_effects.drain(..) {
        if let rebellion_core::effects::GameEffect::CharacterKilled { character } = effect {
            if let Some(c) = world.characters.get(character) {
                // Notification 0x1a, Character Health.
                msg_log.push(filed(
                    GameMessage::new(
                        tick,
                        format!("{} has been killed.", c.name),
                        MessageCategory::Event,
                    )
                    .with_picture_if(Some(pictures::killed(
                        pictures::character_side(world, character),
                    ))),
                    MessageRail::Mission,
                    character_audience(world, character),
                ));
            }
        }
    }
    // Notification 0x182, System Destroyed by the Death Star
    // (FUN_004950d0), for both sides.
    let (title, text) =
        rebellion_render::story_messages::death_star_destroyed_texts(&name, holder, message_texts)
            .map_or_else(
                || (None, format!("{name} DESTROYED by Death Star superlaser!")),
                |(title, body)| (Some(title), body),
            );
    let mut message = GameMessage::new(tick, text, MessageCategory::Combat);
    if let Some(title) = title {
        message = message.with_title(title);
    }
    msg_log.push(filed(message, MessageRail::Mission, Some(RailAudience::Both)));
}

#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and UI inputs at this existing integration boundary."
)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
fn apply_ai_actions(
    actions: &[AIAction],
    ai_state: &mut AIState,
    mission_state: &mut MissionState,
    mfg_state: &mut ManufacturingState,
    movement_state: &mut MovementState,
    troop_transport_state: &mut TroopTransportState,
    production_state: &mut ProductionState,
    world: &mut GameWorld,
    log: &mut MessageLog,
    tick: u64,
    bombard: &mut BombardEnv,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
    #[cfg(not(target_arch = "wasm32"))] audio_vol: &AudioVolumeState,
) {
    for action in actions {
        match action {
            AIAction::DispatchMission {
                kind,
                team,
                decoys,
                target_system,
                target_character,
                target_object,
            } => {
                let ai_faction = ai_state.faction.unwrap_or(AiFaction::Empire);
                let request = rebellion_core::missions::MissionRequest {
                    kind: *kind,
                    faction: ai_faction.as_mission_faction(),
                    team: team.clone(),
                    decoys: decoys.clone(),
                    target_system: *target_system,
                    target_character: *target_character,
                    target_object: *target_object,
                    tick,
                };
                let Ok(id) = mission_state.dispatch_guarded(request, world) else {
                    continue;
                };
                for character in team.iter().chain(decoys).filter_map(|m| m.character()) {
                    ai_state.mark_busy(character);
                }
                if let Some(&lead) = team.first() {
                    ai_state.bind_mission(*kind, lead, id);
                }
                let faction_name = match ai_faction {
                    AiFaction::Alliance => "Alliance",
                    AiFaction::Empire => "Empire",
                };
                log.push(GameMessage::at_system(
                    tick,
                    format!("{faction_name} dispatched {kind:?} mission"),
                    MessageCategory::Ai,
                    *target_system,
                ));
            }
            AIAction::AbortMission { mission_id } => {
                mission_state.abort(*mission_id);
            }
            AIAction::EnqueueProduction { system, kind, cost } => {
                mfg_state.enqueue(*system, QueueItem::new(*kind, *cost));
            }
            AIAction::Scrap { team } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                let scrapped = rebellion_core::scrap::scrap_team(
                    world,
                    production_state,
                    side.as_faction(),
                    team,
                );
                // A scrapped ship's regiments beyond the room left are lost.
                troop_transport_state.destroy_untransportable_cargo(world);
                if let Ok(done) = scrapped {
                    for unit in done {
                        log.push(GameMessage::at_system(
                            tick,
                            format!(
                                "{} scrapped a unit (refund {})",
                                match side {
                                    AiFaction::Alliance => "Alliance",
                                    AiFaction::Empire => "Empire",
                                },
                                unit.refund
                            ),
                            MessageCategory::Ai,
                            unit.system,
                        ));
                    }
                }
            }
            AIAction::MoveRegiments { troops, to } => {
                let blockaded = rebellion_core::blockade::BlockadeSystem::blockaded_now(world);
                for &troop in troops {
                    let _ = troop_transport_state.move_regiment(
                        world,
                        movement_state,
                        &blockaded,
                        troop,
                        rebellion_core::troop_transport::RegimentTarget::System(*to),
                        tick,
                    );
                }
            }
            AIAction::BoardRegiments { troops, fleet } => {
                let blockaded = rebellion_core::blockade::BlockadeSystem::blockaded_now(world);
                for &troop in troops {
                    let _ = troop_transport_state.move_regiment(
                        world,
                        movement_state,
                        &blockaded,
                        troop,
                        rebellion_core::troop_transport::RegimentTarget::Fleet(*fleet),
                        tick,
                    );
                }
            }
            AIAction::MoveSquadrons {
                class,
                count,
                from,
                to,
            } => {
                if let Some(alliance) = rebellion_core::squadrons::is_alliance(world, *class) {
                    let _ = troop_transport_state
                        .squadrons
                        .move_squadrons(world, alliance, *class, *count, *from, *to, tick);
                }
            }
            AIAction::MoveSquadronTeam {
                squadrons,
                from,
                to,
            } => {
                for &(class, count) in squadrons {
                    if let Some(alliance) = rebellion_core::squadrons::is_alliance(world, class) {
                        let _ = troop_transport_state
                            .squadrons
                            .move_squadrons(world, alliance, class, count, *from, *to, tick);
                    }
                }
            }
            AIAction::MoveCharacters { characters, to } => {
                for &character in characters {
                    rebellion_core::missions::move_member(
                        world,
                        rebellion_core::missions::MissionMember::Character(character),
                        Some(*to),
                    );
                }
            }
            AIAction::Bombard {
                fleets,
                system,
                kind,
            } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                if world.systems.contains_key(*system) {
                    let outcome = bombard
                        .bombard_reported(
                            world,
                            troop_transport_state,
                            side.as_faction(),
                            fleets,
                            *system,
                            *kind,
                        )
                        .clone();
                    log_bombardment(world, *system, &outcome, tick, log);
                }
            }
            AIAction::Assault { fleets, system } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                if world.systems.contains_key(*system) {
                    let outcome = bombard
                        .assault(
                            world,
                            troop_transport_state,
                            side.as_faction(),
                            fleets,
                            *system,
                        )
                        .clone();
                    log_assault(world, *system, &outcome, tick, log);
                }
            }
            AIAction::JoinFleet { fleet, into } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                let to = world.fleets.get(*into).map(|f| f.location);
                let joined = rebellion_core::fleet_join::join_fleet(
                    world,
                    movement_state,
                    troop_transport_state,
                    &rebellion_core::fleet_join::FleetMover::Fleet(*fleet),
                    *into,
                    matches!(side, AiFaction::Alliance),
                );
                if let (Ok(_), Some(to)) = (joined, to) {
                    log.push(GameMessage::at_system(
                        tick,
                        format!(
                            "{} fleet joining a fleet",
                            match side {
                                AiFaction::Alliance => "Alliance",
                                AiFaction::Empire => "Empire",
                            }
                        ),
                        MessageCategory::Ai,
                        to,
                    ));
                }
            }
            AIAction::MoveShips { fleet, ships, to } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                let Some(roster) = rebellion_core::fleet_join::roster(world, *fleet) else {
                    continue;
                };
                let moved = rebellion_core::fleet_join::move_ships_to_system(
                    world,
                    movement_state,
                    troop_transport_state,
                    *fleet,
                    ships,
                    roster,
                    *to,
                    matches!(side, AiFaction::Alliance),
                );
                if moved.is_ok() {
                    log.push(GameMessage::at_system(
                        tick,
                        format!(
                            "{} moving {} ship(s) to system",
                            match side {
                                AiFaction::Alliance => "Alliance",
                                AiFaction::Empire => "Empire",
                            },
                            ships.len()
                        ),
                        MessageCategory::Ai,
                        *to,
                    ));
                }
            }
            AIAction::CreateFleet { fleet, ships, to } => {
                let Some(side) = ai_state.faction else {
                    continue;
                };
                let Some(roster) = rebellion_core::fleet_join::roster(world, *fleet) else {
                    continue;
                };
                let is_alliance = matches!(side, AiFaction::Alliance);
                if world.fleets.get(*fleet).is_some_and(|f| f.location == *to) {
                    let _ = rebellion_core::fleet_join::create_fleet(
                        world,
                        movement_state,
                        troop_transport_state,
                        *fleet,
                        ships,
                        roster,
                        is_alliance,
                    );
                } else {
                    let _ = rebellion_core::fleet_join::move_ships_to_system(
                        world,
                        movement_state,
                        troop_transport_state,
                        *fleet,
                        ships,
                        roster,
                        *to,
                        is_alliance,
                    );
                }
            }
            AIAction::SetDestination {
                system,
                area,
                destination,
            } => {
                mfg_state.set_destination(*system, *area, *destination);
            }
            AIAction::Build {
                system,
                kind,
                cost,
                count,
            } => {
                for _ in 0..*count {
                    mfg_state.enqueue(*system, QueueItem::new(*kind, *cost));
                }
            }
            AIAction::MoveFleet {
                fleet,
                to_system,
                reason,
                troops,
            } => {
                let transit = world.fleets.get(*fleet).and_then(|fleet| {
                    rebellion_core::movement::fleet_transit_ticks(
                        fleet,
                        world,
                        fleet.location,
                        *to_system,
                    )
                    .map(|ticks| (ticks, fleet.is_alliance))
                });
                if let Some((transit, is_alliance)) = transit {
                    let embarked = troops.is_empty()
                        || troop_transport_state.embark(world, *fleet, troops).is_ok();
                    if embarked
                        && begin_fleet_transit(movement_state, world, *fleet, *to_system, transit)
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            audio_engine.play_sfx(SfxKind::FleetDeparture, audio_vol);
                        }
                        let reason_str = match reason {
                            FleetMoveReason::Attack => "attack",
                            FleetMoveReason::Reinforce => "reinforce",
                        };
                        log.push(GameMessage::at_system(
                            tick,
                            format!(
                                "{} fleet moving to system ({}){}",
                                if is_alliance { "Alliance" } else { "Empire" },
                                reason_str,
                                if troops.is_empty() {
                                    String::new()
                                } else {
                                    format!(" with {} regiment(s)", troops.len())
                                },
                            ),
                            MessageCategory::Ai,
                            *to_system,
                        ));
                    } else if embarked && !troops.is_empty() {
                        let origin = world.fleets.get(*fleet).map(|value| value.location);
                        if let Some(origin) = origin {
                            let _ = troop_transport_state.disembark_all(world, *fleet, origin);
                        }
                    }
                }
            }
        }
    }
}

fn draw_fullscreen_texture(texture: &Texture2D) {
    let texture_width = texture.width();
    let texture_height = texture.height();
    if texture_width <= 0.0 || texture_height <= 0.0 {
        return;
    }

    let scale = (screen_width() / texture_width).min(screen_height() / texture_height);
    let dest_width = texture_width * scale;
    let dest_height = texture_height * scale;
    let x = (screen_width() - dest_width) * 0.5;
    let y = (screen_height() - dest_height) * 0.5;

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

/// File a report on the Message Index rail when some side receives it.
fn filed(message: GameMessage, rail: MessageRail, audience: Option<RailAudience>) -> GameMessage {
    match audience {
        Some(audience) => message.on_rail(rail, audience),
        None => message,
    }
}

/// The side that controls `system`, whose Message Index receives its reports.
fn system_audience(world: &GameWorld, system: SystemKey) -> Option<RailAudience> {
    match world.systems.get(system)?.control.faction()? {
        Faction::Alliance => Some(RailAudience::Alliance),
        Faction::Empire => Some(RailAudience::Empire),
        Faction::Neutral => None,
    }
}

/// The side a character serves, whose Message Index receives its reports.
fn character_audience(world: &GameWorld, character: CharacterKey) -> Option<RailAudience> {
    let character = world.characters.get(character)?;
    if character.is_alliance {
        Some(RailAudience::Alliance)
    } else if character.is_empire {
        Some(RailAudience::Empire)
    } else {
        None
    }
}

fn victory_winner_is_alliance(outcome: &rebellion_core::victory::VictoryOutcome) -> bool {
    match outcome {
        rebellion_core::victory::VictoryOutcome::HqCaptured { winner, .. }
        | rebellion_core::victory::VictoryOutcome::HqDestroyed { winner, .. } => {
            *winner == Faction::Alliance
        }
        rebellion_core::victory::VictoryOutcome::DeathStarVictory { .. } => false,
    }
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

fn open_cutscene(
    path: &Path,
    volume: f32,
    msg_log: &mut MessageLog,
    tick: u64,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
) -> Option<VideoPlayer> {
    #[cfg(not(target_arch = "wasm32"))]
    audio_engine.stop_music();

    match VideoPlayer::open(path, volume) {
        Ok(player) => {
            macroquad::logging::info!("[cutscene] opened path={}", path.display());
            Some(player)
        }
        Err(VideoError::NotDecoded { .. }) => {
            let message =
                "cutscene skipped — run scripts/decode-cutscenes.sh to enable".to_string();
            eprintln!("[cutscene] {message}");
            msg_log.push(GameMessage::new(tick, message, MessageCategory::Event));
            None
        }
        Err(error) => {
            let message = format!("cutscene skipped — {error}");
            eprintln!("[cutscene] {message}");
            msg_log.push(GameMessage::new(tick, message, MessageCategory::Event));
            None
        }
    }
}

#[cfg(test)]
mod tactical_ground_tests {
    use super::*;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey};
    use rebellion_core::world::{ControlKind, FighterEntry, Fleet, Sector, ShipInstance, System};

    #[test]
    fn trench_run_outcomes_route_to_their_source_movies() {
        assert_eq!(
            trench_run_cutscene_path(TacticalTrenchRunOutcome::Success),
            "assets/references/ref-videos/201.webm"
        );
        assert_eq!(
            trench_run_cutscene_path(TacticalTrenchRunOutcome::Failure),
            "assets/references/ref-videos/202.webm"
        );
    }

    #[test]
    fn each_victory_outcome_names_its_winning_faction() {
        let system = rebellion_core::ids::SystemKey::default();
        assert!(victory_winner_is_alliance(
            &rebellion_core::victory::VictoryOutcome::HqCaptured {
                winner: Faction::Alliance,
                loser: Faction::Empire,
                hq_system: system,
            }
        ));
        assert!(!victory_winner_is_alliance(
            &rebellion_core::victory::VictoryOutcome::HqCaptured {
                winner: Faction::Empire,
                loser: Faction::Alliance,
                hq_system: system,
            }
        ));
        assert!(!victory_winner_is_alliance(
            &rebellion_core::victory::VictoryOutcome::HqDestroyed {
                winner: Faction::Empire,
                loser: Faction::Alliance,
                hq_system: system,
            }
        ));
        assert!(!victory_winner_is_alliance(
            &rebellion_core::victory::VictoryOutcome::DeathStarVictory {
                target_system: system,
            }
        ));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "The tactical result regression keeps its opposing fleets and exact roster assertions together."
    )]
    fn tactical_space_results_preserve_exact_capital_and_fighter_roster_slots() {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(1),
            name: "Test Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(2),
            name: "Test System".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });
        let ship_class = CapitalShipKey::default();
        let fighter_class = FighterKey::default();
        let mut preexisting_dead_ship = ShipInstance::new(ship_class, 100, true);
        preexisting_dead_ship.hull_current = 0;
        preexisting_dead_ship.alive = false;
        let attacker = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![
                preexisting_dead_ship,
                ShipInstance::new(ship_class, 100, true),
                ShipInstance::new(ship_class, 100, true),
            ],
            fighters: vec![
                FighterEntry {
                    class: fighter_class,
                    count: 12,
                },
                FighterEntry {
                    class: fighter_class,
                    count: 12,
                },
            ],
            characters: vec![],
            is_alliance: true,
            has_death_star: true,
            special_forces: Vec::new(),
        });
        let defender = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(ship_class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets = vec![attacker, defender];
        let session = rebellion_render::BattleSession {
            system,
            system_name: "Test System".into(),
            system_picture_id: 1,
            attacker_fleet: attacker,
            defender_fleet: defender,
            attacker_is_alliance: true,
            player_is_attacker: true,
            phase: rebellion_render::BattlePhase::Results,
            ships: vec![
                test_tactical_ship(ship_class, true, 1, 43, true),
                test_tactical_ship(ship_class, true, 2, 0, false),
                test_tactical_ship(ship_class, false, 0, 91, true),
            ],
            fighters: vec![
                rebellion_render::tactical_view::TacticalFighter {
                    class_key: fighter_class,
                    identity: rebellion_render::tactical_view::TacticalObjectIdentity {
                        class_dat_id: DatId::new(3),
                        fleet_roster_index: 0,
                        is_alliance: true,
                    },
                    tactical_resource: rebellion_render::fighter_tactical_resource(DatId::new(3)),
                    source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    sublight_engine_power: 0.0,
                    fleet_fighter_index: 0,
                    fleet_squadron_index: 0,
                    name: "Fighter".into(),
                    x: 0.0,
                    y: 0.0,
                    squad_count: 12,
                    hull_current: 96.0,
                    hull_max: 96.0,
                    shield: 0.0,
                    shield_max: 0.0,
                    weapon_arc: rebellion_render::tactical_view::TacticalWeaponArc::default(),
                    weapon_ranges:
                        rebellion_render::tactical_view::TacticalWeaponRanges::default(),
                    torpedo_strength: 0,
                    torpedo_range: 0.0,
                    maneuverability: 0,
                    is_attacker: true,
                    alive: true,
                    fighter_group: 0,
                    selected: false,
                    order: rebellion_render::tactical_view::TacticalOrder::None,
                    tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
                    attack_target: None,
                    escort_target: None,
                    recovery_state: rebellion_render::tactical_view::TacticalFighterRecoveryState::AwaitingCarrier,
                    recovery_target: None,
                },
                rebellion_render::tactical_view::TacticalFighter {
                    class_key: fighter_class,
                    identity: rebellion_render::tactical_view::TacticalObjectIdentity {
                        class_dat_id: DatId::new(3),
                        fleet_roster_index: 1,
                        is_alliance: true,
                    },
                    tactical_resource: rebellion_render::fighter_tactical_resource(DatId::new(3)),
                    source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    sublight_engine_power: 0.0,
                    fleet_fighter_index: 1,
                    fleet_squadron_index: 0,
                    name: "Fighter".into(),
                    x: 0.0,
                    y: 0.0,
                    squad_count: 0,
                    hull_current: 0.0,
                    hull_max: 96.0,
                    shield: 0.0,
                    shield_max: 0.0,
                    weapon_arc: rebellion_render::tactical_view::TacticalWeaponArc::default(),
                    weapon_ranges:
                        rebellion_render::tactical_view::TacticalWeaponRanges::default(),
                    torpedo_strength: 0,
                    torpedo_range: 0.0,
                    maneuverability: 0,
                    is_attacker: true,
                    alive: true,
                    fighter_group: 1,
                    selected: false,
                    order: rebellion_render::tactical_view::TacticalOrder::None,
                    tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
                    attack_target: None,
                    escort_target: None,
                    recovery_state: rebellion_render::tactical_view::TacticalFighterRecoveryState::AwaitingCarrier,
                    recovery_target: None,
                },
            ],
            death_star: Some(rebellion_render::tactical_view::TacticalDeathStar {
                resource: rebellion_render::DEATH_STAR_TACTICAL_RESOURCE,
                is_attacker: true,
                is_alliance: true,
                source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                hull: 0.0,
                laser_charge: 0.0,
                shielded: false,
                destroyed: true,
                action_committed: false,
            }),
            death_star_beam: None,
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating: 1,
            trench_run_pending_outcome: None,
            trench_run_participants: vec![],
            trench_run_chatter: [0x13d; 9],
            trench_run_chatter_cursor: 0,
            trench_run_cinematic_pending: false,
            navigation_sets: std::array::from_fn(|_| Vec::new()),
            source_layout: rebellion_render::tactical_view::OriginalTacticalLayout::default(),
            selected_ship: None,
            selected_fighter_group: None,
            placement_confirmed: true,
            start_tick: 1,
            combat_tick: 1,
            tactical_rng: rebellion_render::OriginalTacticalRng::new(0x5eed_ba77),
            weapon_effects: vec![],
            impact_effects: vec![],
            pending_audio_cues: vec![],
            pending_voice_cues: vec![],
            field_effects: vec![],
            subsystem_repairs: vec![],
            paused: false,
            combat_speed: 1,
            step_accumulator: 0.0,
            winner: Some(rebellion_render::CombatWinner::Attacker),
            diagnostic_log: Vec::new(),
        };

        let battle_return =
            tactical_flow::apply_results(&session, &mut world, &mut TroopTransportState::default());

        assert_eq!(world.fleets[attacker].capital_ships.len(), 1);
        assert_eq!(world.fleets[attacker].capital_ships[0].hull_current, 43);
        assert_eq!(world.fleets[attacker].fighters[0].count, 1);
        assert_eq!(world.fleets[attacker].fighters[1].count, 0);
        assert!(!world.fleets[attacker].has_death_star);
        assert_eq!(world.fleets[defender].capital_ships[0].hull_current, 91);
        assert_eq!(
            battle_return.winner,
            Some(rebellion_render::CombatWinner::Attacker)
        );
        assert_eq!(battle_return.winner_fleet, Some(attacker));
        assert!(battle_return.player_won);
    }

    fn test_tactical_ship(
        class_key: CapitalShipKey,
        is_attacker: bool,
        fleet_ship_index: usize,
        hull_current: i32,
        alive: bool,
    ) -> rebellion_render::tactical_view::TacticalShip {
        rebellion_render::tactical_view::TacticalShip {
            class_key,
            identity: rebellion_render::tactical_view::TacticalObjectIdentity {
                class_dat_id: DatId::new(1),
                fleet_roster_index: fleet_ship_index,
                is_alliance: is_attacker,
            },
            tactical_resource: rebellion_render::capital_ship_tactical_resource(DatId::new(1)),
            death_star_resource: rebellion_render::death_star_tactical_resource(DatId::new(1)),
            source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            source_forward: rebellion_render::tactical_view::TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: if is_attacker { 1.0 } else { -1.0 },
            },
            source_desired_forward: rebellion_render::tactical_view::TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: if is_attacker { 1.0 } else { -1.0 },
            },
            source_velocity: rebellion_render::tactical_view::TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            source_waypoint: None,
            navigation_route: Vec::new(),
            source_collision_envelope: None,
            name: "Ship".into(),
            x: 0.0,
            y: 0.0,
            hull_current,
            hull_max: 100,
            shield: 0,
            shield_max: 0,
            subsystem_condition: rebellion_render::tactical_view::TacticalSubsystemCondition {
                shields: 0,
                weapons: 0,
                tractor: 0,
                engines: 0,
                hyperdrive: 0,
            },
            subsystem_capacity: rebellion_render::tactical_view::TacticalSubsystemCapacity {
                shields: false,
                weapons: false,
                tractor: false,
                engines: false,
                hyperdrive: 0,
            },
            subsystem_damage: rebellion_render::tactical_view::TacticalSubsystemDamage::default(),
            sublight_engine_power: 0.0,
            maneuverability: 0,
            tractor_beam_power: 0.0,
            fighter_capacity: 0,
            maneuver_state_value: None,
            damage_control: 0,
            is_attacker,
            alive,
            selected: false,
            order: rebellion_render::tactical_view::TacticalOrder::None,
            tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
            task_force: u8::try_from(fleet_ship_index.min(7)).unwrap_or(7),
            fleet_ship_index,
            sprite_id: None,
            weapon_arcs: [rebellion_render::tactical_view::TacticalWeaponArc::default(); 4],
            weapon_ranges: rebellion_render::tactical_view::TacticalWeaponRanges::default(),
            weapon_recharge_rate: 0.0,
            shield_recharge_rate: 0.0,
            shield_recharge_carry: 0.0,
            weapon_recharge_queue: Vec::new(),
            attack_target: None,
            manual_targets: Vec::new(),
            escort_target: None,
            contents: Vec::new(),
            retreating: false,
            retreat_progress: 0.0,
            retreated: false,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod options_save_tests {
    #[test]
    fn unreadable_native_saves_remain_occupied_and_unchanged() {
        let root =
            std::env::temp_dir().join(format!("options-corrupt-save-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = rebellion_data::save::slot_path(&root, 9);
        std::fs::write(&path, b"corrupt save fixture").unwrap();
        let slots = super::read_save_slots(&root);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot, 9);
        assert_eq!(slots[0].name, "Unreadable save");
        assert_eq!(std::fs::read(&path).unwrap(), b"corrupt save fixture");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod fleet_move_tests {
    use super::*;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::DatId;
    use rebellion_core::world::{CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System};

    fn system(sector: rebellion_core::ids::SectorKey, x: u16) -> System {
        System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0000),
            name: format!("Sys{x}"),
            sector,
            x,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        }
    }

    /// Two Alliance fleets at one system, and a second system 40 away.
    // FUN_004ff8e0 (a fleet's orders) and FUN_00507290 (the fleet icon's):
    // the order's team is the fleet, or the side's fleets at the icon's
    // system, and its target the system they orbit.
    #[test]
    fn the_bombardment_team_is_the_fleet_or_the_sides_fleets_at_the_icon() {
        let (mut world, fleets, _) = world_with_two_fleets();
        let origin = world.fleets[fleets[0]].location;
        let enemy = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[origin].fleets.push(enemy);
        let icon = |quadrant| {
            Some(MenuObject::SystemIcon {
                system: origin,
                quadrant,
            })
        };
        assert_eq!(
            bombard_team(&world, Some(MenuObject::Fleet(fleets[1])), true),
            Some((vec![fleets[1]], origin))
        );
        assert_eq!(
            bombard_team(&world, icon(Quadrant::Fleets), true),
            Some((fleets.clone(), origin))
        );
        assert_eq!(
            bombard_team(&world, icon(Quadrant::Fleets), false),
            Some((vec![enemy], origin))
        );
        assert_eq!(bombard_team(&world, icon(Quadrant::Defenses), true), None);
        assert_eq!(bombard_team(&world, None, true), None);
    }

    fn world_with_two_fleets() -> (GameWorld, Vec<FleetKey>, SystemKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Test".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let origin = world.systems.insert(system(sector, 0));
        let destination = world.systems.insert(system(sector, 40));
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            name: "TestShip".into(),
            is_alliance: true,
            hull: 100,
            hyperdrive: 80,
            ..CapitalShipClass::default()
        });
        let fleets = (0..2)
            .map(|_| {
                let fleet = world.fleets.insert(Fleet {
                    location: origin,
                    capital_ships: vec![ShipInstance::new(class, 100, true)],
                    fighters: vec![],
                    characters: vec![],
                    is_alliance: true,
                    has_death_star: false,
                    special_forces: Vec::new(),
                });
                world.systems[origin].fleets.push(fleet);
                fleet
            })
            .collect();
        (world, fleets, destination)
    }

    struct Issued {
        actions: Vec<PanelAction>,
        confirmation: MoveConfirmationState,
        log: MessageLog,
    }

    fn issue(world: &GameWorld, fleets: &[FleetKey], confirmed: bool, to: SystemKey) -> Issued {
        issue_onto(world, fleets, confirmed, ReleaseTarget::System(to))
    }

    fn issue_onto(
        world: &GameWorld,
        fleets: &[FleetKey],
        confirmed: bool,
        target: ReleaseTarget,
    ) -> Issued {
        let movement = MovementState::new();
        let blockaded = HashSet::new();
        let mut issued = Issued {
            actions: Vec::new(),
            confirmation: MoveConfirmationState::default(),
            log: MessageLog::new(8),
        };
        issue_fleet_move(
            &FleetMoveContext {
                world,
                movement: &movement,
                blockaded: &blockaded,
                faction: MissionFaction::Alliance,
                tick: 0,
            },
            fleets,
            confirmed,
            target,
            &mut issued.log,
            &mut issued.confirmation,
            &mut issued.actions,
        );
        issued
    }

    fn dispatched(actions: &[PanelAction]) -> Vec<FleetKey> {
        actions
            .iter()
            .filter_map(|action| match action {
                PanelAction::DispatchFleet { fleet, .. } => Some(*fleet),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_fleet_icons_move_departs_every_fleet_of_the_team() {
        // FUN_00486fb0 -> FUN_00487740: a valid Move that need not ask
        // submits the order, and every sub-order departs.
        let (world, fleets, destination) = world_with_two_fleets();
        let issued = issue(&world, &fleets, false, destination);
        assert_eq!(dispatched(&issued.actions), fleets);
        assert!(!issued.confirmation.is_open());
    }

    #[test]
    fn a_fleet_icons_confirmed_move_asks_once_for_the_whole_team() {
        // FUN_00487cc0 asks for 0x202; the window lists a line per member
        // (FUN_0053c2e0) and departs nothing until the checkmark.
        let (world, fleets, destination) = world_with_two_fleets();
        let issued = issue(&world, &fleets, true, destination);
        assert!(issued.actions.is_empty());
        let window = issued.confirmation.confirmation().expect("the window opens");
        assert_eq!(window.fleets, fleets);
        assert_eq!(window.lines.len(), 2);
        // FUN_0049a350: each line counts the member's days to go.
        assert!(window.lines.iter().all(|(_, days)| *days > 0));
    }

    #[test]
    fn a_fleet_icons_move_onto_a_fleet_in_its_own_system_joins_each_member() {
        // FUN_00486fb0: a target fleet makes each sub-order a join
        // (FUN_004ffc90), which needs no departure where the target lies.
        let (mut world, fleets, _) = world_with_two_fleets();
        let origin = world.fleets[fleets[0]].location;
        let target = world.fleets.insert(world.fleets[fleets[0]].clone());
        world.systems[origin].fleets.push(target);
        let issued = issue_onto(
            &world,
            &fleets,
            false,
            ReleaseTarget::Fleet {
                fleet: target,
                system: origin,
            },
        );
        let joined: Vec<FleetKey> = issued
            .actions
            .iter()
            .filter_map(|action| match action {
                PanelAction::JoinFleet {
                    mover: FleetMover::Fleet(fleet),
                    target: onto,
                } if *onto == target => Some(*fleet),
                _ => None,
            })
            .collect();
        assert_eq!(joined, fleets);
        assert!(issued.log.messages().is_empty());
    }

    #[test]
    fn an_empty_team_moving_onto_a_fleet_is_refused() {
        // FUN_0053c4b0 refuses a move with no members (1/0x16) before any
        // sub-order runs.
        let (world, fleets, _) = world_with_two_fleets();
        let origin = world.fleets[fleets[0]].location;
        let issued = issue_onto(
            &world,
            &[],
            false,
            ReleaseTarget::Fleet {
                fleet: fleets[0],
                system: origin,
            },
        );
        assert!(issued.actions.is_empty());
        assert_eq!(issued.log.messages().len(), 1);
    }

    #[test]
    fn a_bands_stop_and_destination_belong_to_its_systems_holder() {
        // Manual p. 83-84: a construction yard's band takes Destination and
        // Stop; Stop clears what the band is building. Either side holds
        // its own bands.
        for (holder, other) in [
            (MissionFaction::Alliance, MissionFaction::Empire),
            (MissionFaction::Empire, MissionFaction::Alliance),
        ] {
            let mut world = GameWorld::default();
            let control = rebellion_core::world::ControlKind::Controlled(match holder {
                MissionFaction::Alliance => Faction::Alliance,
                MissionFaction::Empire => Faction::Empire,
            });
            let system = world.systems.insert(rebellion_core::world::System {
                last_arrival: None,
                dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
                name: "Bortras".into(),
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
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control,
            });
            let class =
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        is_alliance: true,
                        is_empire: true,
                        ..rebellion_core::world::CapitalShipClass::default()
                    });
            let mut manufacturing = ManufacturingState::new();
            let band = Some(MenuObject::Producer {
                system,
                area: rebellion_core::manufacturing::ProductionArea::Shipyard,
            });
            let gates = |world: &GameWorld, manufacturing: &ManufacturingState, player| {
                order_gates(
                    band,
                    world,
                    &MissionState::new(),
                    &MovementState::new(),
                    &TroopTransportState::new(),
                    manufacturing,
                    &rebellion_core::research::ResearchState::new(),
                    player,
                )
            };
            let idle = gates(&world, &manufacturing, holder);
            assert!(idle.destination && idle.build && !idle.stop);
            manufacturing.build(
                system,
                &QueueItem::new(
                    rebellion_core::manufacturing::BuildableKind::CapitalShip(class),
                    10,
                ),
                1,
            );
            let building = gates(&world, &manufacturing, holder);
            assert!(building.destination && building.stop);
            let foreign = gates(&world, &manufacturing, other);
            assert!(!foreign.destination && !foreign.build && !foreign.stop);
            // With nothing its side may build, Build stays off.
            world.capital_ship_classes[class].research_order = 1;
            assert!(!gates(&world, &manufacturing, holder).build);
        }
    }

    #[test]
    fn a_fleet_icons_move_is_enabled_only_where_its_side_has_fleets() {
        // FUN_0051d990 asks each order's +0x18; Move on the fleet icon
        // (kind 0x10) needs a team (FUN_0053c4b0) that may all move.
        let (world, fleets, destination) = world_with_two_fleets();
        let origin = world.fleets[fleets[0]].location;
        let gate = |system| {
            order_gates(
                Some(MenuObject::SystemIcon {
                    system,
                    quadrant: Quadrant::Fleets,
                }),
                &world,
                &MissionState::new(),
                &MovementState::new(),
                &TroopTransportState::new(),
                &ManufacturingState::new(),
                &rebellion_core::research::ResearchState::new(),
                MissionFaction::Alliance,
            )
            .fleet_move
        };
        assert!(gate(origin));
        assert!(!gate(destination));
    }

    #[test]
    fn one_refused_fleet_keeps_the_whole_team_home() {
        // FUN_0053c1a0 runs every sub-order's validator; FUN_00487c90 reports
        // the refusal and nothing departs.
        let (mut world, fleets, destination) = world_with_two_fleets();
        world.fleets[fleets[1]].capital_ships.clear();
        let issued = issue(&world, &fleets, false, destination);
        assert!(issued.actions.is_empty());
        assert!(!issued.confirmation.is_open());
        assert_eq!(issued.log.messages().len(), 1);
    }
}

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
mod character_health_tests {
    use super::*;

    #[test]
    fn an_injury_posts_injured_and_full_healing_posts_healed() {
        // FUN_0048fdf0 (notification 0x106): TEXTSTRA 0x7160/0x7161 with a
        // non-zero +0x94, 0x7162/0x7163 at 0; pictures 0x41a and 0x41b.
        let mut world = GameWorld::default();
        let character = world.characters.insert(rebellion_core::world::Character {
            name: "Han Solo".into(),
            is_alliance: true,
            injury: 7,
            ..rebellion_core::world::Character::default()
        });
        let mut log = MessageLog::new(8);
        push_character_health(&mut log, &world, character, 3);
        world.characters[character].injury = 0;
        push_character_health(&mut log, &world, character, 9);
        let messages = log.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0].text,
            "Han Solo Injured: Han Solo has been injured."
        );
        assert_eq!(messages[0].picture.map(|p| p.background), Some(0x41a));
        assert_eq!(
            messages[1].text,
            "Han Solo Healed: Han Solo has been healed of all injuries."
        );
        assert_eq!(messages[1].picture.map(|p| p.background), Some(0x41b));
        // The Healed picture carries the character's portrait
        // (FUN_004c5000), found by its record and major flag: Vader is an
        // Empire major character (MJCHARSD 641).
        let vader = world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x281),
            name: "Darth Vader".into(),
            is_empire: true,
            is_major: true,
            ..rebellion_core::world::Character::default()
        });
        push_character_health(&mut log, &world, vader, 10);
        let portrait = log.messages()[2].picture.and_then(|p| p.foreground);
        assert!(portrait.is_some());
        assert_eq!(
            portrait,
            rebellion_render::story_messages::character_portrait(
                rebellion_core::ids::DatId::new(0x281),
                true
            )
        );
    }

    #[test]
    fn a_detection_injury_posts_character_injured() {
        // FUN_0048fdf0: an injury the detection deals (advance effects)
        // posts 0x7160/0x7161; a zero roll posts nothing.
        let mut world = GameWorld::default();
        let han = world.characters.insert(rebellion_core::world::Character {
            name: "Han Solo".into(),
            is_alliance: true,
            ..rebellion_core::world::Character::default()
        });
        let wedge = world.characters.insert(rebellion_core::world::Character {
            name: "Wedge Antilles".into(),
            is_alliance: true,
            ..rebellion_core::world::Character::default()
        });
        let effects = [
            MissionEffect::CharacterInjured {
                character: wedge,
                injury: 0,
            },
            MissionEffect::CharacterInjured {
                character: han,
                injury: 4,
            },
        ];
        rebellion_core::missions::apply_advance_effects(&mut world, &effects);
        let mut log = MessageLog::new(8);
        push_advance_injuries(&mut log, &world, &effects, 5);
        assert_eq!(world.characters[han].injury, 4);
        assert_eq!(log.messages().len(), 1);
        assert_eq!(
            log.messages()[0].text,
            "Han Solo Injured: Han Solo has been injured."
        );
    }

    #[test]
    fn a_force_discovery_posts_future_jedi_discovered_to_the_trainers_side() {
        // FUN_0048f1e0 (notification 0x362): 0x7178 with 0x717a below Force
        // 100 (GNPRTB 2581, absent here, so 0), on the Empire's rail only.
        let mut world = GameWorld::default();
        let vader = world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x281),
            name: "Darth Vader".into(),
            is_empire: true,
            is_major: true,
            is_jedi_trainer: true,
            force_base: 120,
            ..rebellion_core::world::Character::default()
        });
        let klev = world.characters.insert(rebellion_core::world::Character {
            name: "Klev".into(),
            is_empire: true,
            ..rebellion_core::world::Character::default()
        });
        let texts = std::collections::HashMap::from([
            (0x7178, "Future Jedi Discovered".to_owned()),
            (0x7179, "I have detected that {1:1} has the ability to use the Force.  We should consider allowing me to conduct training in the way of the Force.".to_owned()),
        ]);
        let mut log = MessageLog::new(8);
        push_force_discoveries(
            &mut log,
            &world,
            &[rebellion_core::force_discovery::ForceDiscovered {
                character: klev,
                discoverer: vader,
                force_base: None,
                learns_heritage: false,
                tick: 4,
            }],
            &texts,
        );
        let messages = log.messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].title.as_deref(), Some("Future Jedi Discovered"));
        assert!(messages[0].text.starts_with("I have detected that Klev"));
        assert_eq!(messages[0].audience, RailAudience::Empire);
    }
}
