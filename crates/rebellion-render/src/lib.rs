//! Galaxy map rendering and egui UI panels.

pub mod advisor;
pub mod agent_menu;
pub mod assault_summary;
pub mod audio;
pub mod bmp_cache;
pub mod build_selection;
pub mod cockpit;
pub mod defenses_window;
pub mod encyclopedia;
pub mod fleet_finder;
pub mod fleet_movement;
pub mod fleet_registry;
pub mod fleet_window;
pub mod fog;
pub mod game_menu;
pub mod game_options;
pub mod game_speed;
pub mod gid_legend;
pub mod list_scroll_bar;
pub mod main_menu;
pub mod main_menu_destinations;
pub mod manufacturing_window;
pub mod message_index;
pub mod message_log;
pub mod message_pictures;
pub mod minis;
pub mod mission_dialog;
pub mod missions_window;
pub mod move_confirmation;
pub mod object_menu;
pub mod panels;
pub mod personnel_finder;
pub mod quadrant_icons;
pub mod resource_monitors;
pub mod sector_window;
pub mod status_rows;
pub mod status_window;
pub mod story_messages;
pub mod system_window;
mod window_move;
mod tactical_asset_cache;
mod tactical_assets;
pub mod tactical_view;
pub mod targeting;
pub mod theme;
pub mod troop_finder;
pub mod video_player;

use egui_macroquad::egui;
use macroquad::prelude::*;
use rebellion_core::blockade::BlockadeState;
use rebellion_core::dat::ExplorationStatus;
use rebellion_core::economy::EconomyState;
use rebellion_core::ids::SystemKey;
use rebellion_core::manufacturing::{ManufacturingState, ProductionArea};
use rebellion_core::missions::MissionState;
use rebellion_core::movement::MovementState;
use rebellion_core::world::{ControlKind, GameWorld, System};

#[cfg(target_arch = "wasm32")]
pub use advisor::set_advisor_asset_cache;
pub use advisor::{
    advisor_combat_result, advisor_greet, advisor_manufacturing_complete, advisor_mission_result,
    advisor_uprising, draw_advisor, AdvisorFaction, AdvisorMessage, AdvisorPriority, AdvisorState,
};
pub use audio::{draw_audio_controls, AudioVolumeState, MusicContext, MusicTrack, SfxKind};
#[cfg(target_arch = "wasm32")]
pub use bmp_cache::set_bmp_cache;
pub use bmp_cache::{AssetRenderProfile, BmpCache, DllSource};
pub use cockpit::{
    draw_cockpit_background, draw_cockpit_chrome, draw_cockpit_egui_layer,
    handle_cockpit_egui_input, set_cockpit_viewport_clip, strategic_gid_control,
    strategic_primary_controls, CockpitButton, CockpitFaction, CockpitLayout, CockpitState,
    CockpitViewport, GidMode, StrategicControlSpec, STRATEGIC_LOGICAL_HEIGHT,
    STRATEGIC_LOGICAL_WIDTH,
};
pub use defenses_window::{
    draw_defenses_windows, DefensesPage, DefensesWindowAction, DefensesWindowState,
};
#[cfg(target_arch = "wasm32")]
pub use encyclopedia::set_encyclopedia_asset_cache;
pub use encyclopedia::{
    draw_encyclopedia, draw_encyclopedia_index_catalog, draw_encyclopedia_index_shell,
    draw_encyclopedia_topic, open_original_encyclopedia_at, original_encyclopedia_object,
    EncyclopediaState, EncyclopediaTab, OriginalEncyclopediaCatalog, OriginalEncyclopediaEntry,
    OriginalEncyclopediaTopic, ENCYCLOPEDIA_INDEX_HEIGHT, ENCYCLOPEDIA_INDEX_WIDTH,
};
#[cfg(feature = "interface-test-fixtures")]
pub use encyclopedia::{
    draw_encyclopedia_artwork_fixture, draw_encyclopedia_index_catalog_fixture,
    draw_encyclopedia_index_fixture,
};
pub use fleet_movement::{draw_fleet_overlays, hovered_fleet};
pub use fleet_window::{
    draw_fleet_windows, FleetWindowAction, FleetWindowState, FleetWindowTab, FLEET_WINDOW_HEIGHT,
    FLEET_WINDOW_WIDTH,
};
pub use fog::draw_fog_overlay;
pub use game_options::{draw_game_options, GameOptionsAction, GameOptionsOrigin, GameOptionsState};
pub use gid_legend::set_gid_legends;
pub use main_menu::{draw_main_menu, MainMenuAction, MainMenuControl, MainMenuState};
pub use main_menu_destinations::{
    draw_credits, draw_multiplayer_setup, CreditsState, MenuDestinationAction,
    MultiplayerSetupAction, MultiplayerSetupState, MultiplayerTransport,
};
#[cfg(feature = "interface-test-fixtures")]
pub use message_index::draw_message_index_fixture;
pub use message_index::draw_message_index_shell;
pub use message_log::{
    draw_message_log, GameMessage, MessageCategory, MessageLog, MessageLogState, MessageRail,
    RailAudience,
};
pub use missions_window::{
    draw_missions_windows, MissionsTab, MissionsWindowAction, MissionsWindowState,
};
pub use panels::game_setup::{draw_game_setup, Difficulty, GameSetupAction, GameSetupState};
pub use panels::{
    draw_mod_manager, draw_save_load, ModInfo, ModManagerAction, ModManagerState, PanelAction,
    SaveLoadPanelState, SaveSlotInfo,
};
pub use rebellion_core::tactical::{
    BattlePhase, BattleSession, CombatWinner, OriginalTacticalRng, TacticalTrenchRunOutcome,
};
pub use rebellion_core::tactical_resources::{
    capital_ship_tactical_resource, death_star_tactical_resource, fighter_tactical_resource,
    TacticalCapitalShipResource, TacticalDeathStarResource, TacticalFighterResource,
    DEATH_STAR_DAT_ID, DEATH_STAR_TACTICAL_RESOURCE,
};
pub use sector_window::{
    draw_sector_windows, SectorWindowAction, SectorWindowState, SECTOR_WINDOW_HEIGHT,
    SECTOR_WINDOW_WIDTH,
};
pub use system_window::{
    draw_system_windows, SystemWindowAction, SystemWindowState, SystemWindowTab,
    REFERENCE_RAIL_SLOTS, SYSTEM_WINDOW_CLIENT_WIDTH, SYSTEM_WINDOW_HEIGHT, SYSTEM_WINDOW_WIDTH,
};
pub use tactical_asset_cache::{set_tactical_asset_cache, tactical_asset_cache_counts};
#[cfg(not(target_arch = "wasm32"))]
pub use tactical_assets::install_native_tactical_assets;
#[cfg(all(feature = "interface-test-fixtures", not(target_arch = "wasm32")))]
pub use tactical_assets::install_native_tactical_lod_family;
#[cfg(feature = "interface-test-fixtures")]
pub use tactical_assets::TacticalLodView;
pub use tactical_view::{draw_tactical_view, TacticalAction, TacticalState};
pub use video_player::{VideoError, VideoPlayer};

#[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
pub use panels::command_palette::{
    draw_command_palette, CommandPaletteState, InterfaceCommand, PaletteAction,
};

/// Computed camera parameters for overlay rendering.
///
/// Returned by `draw_galaxy_map` so fog/fleet overlays can use matching
/// coordinates without recomputing screen dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraView {
    /// Screen position of DAT coordinate (0, 0).
    pub origin_x: f32,
    pub origin_y: f32,
    /// Screen pixels per DAT unit on each axis; the original map is stretched.
    pub map_scale_x: f32,
    pub map_scale_y: f32,
    /// Size factor for marker and overlay art, including the canvas scale.
    pub zoom: f32,
    /// Player-controlled gameplay zoom, independent of window size.
    pub logical_zoom: f32,
    /// Uniform scale from original 640×480 pixels to screen pixels.
    pub display_scale: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl CameraView {
    /// Convert original DAT coordinates into this aperture's screen space.
    #[must_use]
    pub fn to_screen(self, dat_x: f32, dat_y: f32) -> (f32, f32) {
        (
            self.origin_x + dat_x * self.map_scale_x,
            self.origin_y + dat_y * self.map_scale_y,
        )
    }

    /// Whether a screen-space point is inside the recovered map aperture.
    #[must_use]
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.viewport_x
            && x < self.viewport_x + self.viewport_width
            && y >= self.viewport_y
            && y < self.viewport_y + self.viewport_height
    }

    /// Whether a point lies in or just outside the aperture for draw culling.
    #[must_use]
    pub fn contains_with_margin(self, x: f32, y: f32, margin: f32) -> bool {
        x >= self.viewport_x - margin
            && x <= self.viewport_x + self.viewport_width + margin
            && y >= self.viewport_y - margin
            && y <= self.viewport_y + self.viewport_height + margin
    }

    /// Convert a fixed original-interface pixel measurement to screen pixels.
    #[must_use]
    pub fn scale_pixels(self, logical_pixels: f32) -> f32 {
        logical_pixels * self.display_scale
    }

    /// Apply gameplay zoom and its original clamp before display scaling.
    #[must_use]
    pub fn zoomed_pixels(self, base: f32, min: f32, max: f32) -> f32 {
        (base * self.logical_zoom).clamp(min, max) * self.display_scale
    }
}

/// Canvas pixels per DAT unit on the strategic galaxy map.
///
/// Measured by registering SYSTEMSD positions against original-game frames:
/// squakenet 0129 (Alliance) and 0712 (Empire) place 100 of 100 markers
/// within 2 px, pravus 0157 places 143 of 150. The map is stretched, not
/// uniformly scaled, and the scale is the same for every galaxy size.
pub const GALAXY_MAP_SCALE: (f32, f32) = (0.5946, 0.4286);

/// Canvas position of DAT (0, 0) relative to the map content origin.
/// Measured from the same frames as [`GALAXY_MAP_SCALE`]: markers sit at
/// canvas (28.7, 31.7) in the Alliance cockpit, (7.7, 6.7) past its origin.
pub const GALAXY_MAP_OFFSET: (f32, f32) = (7.7, 6.7);

/// Canvas origin of the galaxy map content (starfield and markers).
///
/// `FUN_00425d00` offsets GID markers by (21, 25) in the Alliance cockpit and
/// (84, 27) in the Imperial one. Registering the starfield against squakenet
/// frames 0129 and 0712 puts its top-left corner at those same points.
#[must_use]
pub const fn galaxy_content_origin(faction: CockpitFaction) -> (f32, f32) {
    match faction {
        CockpitFaction::Alliance => (21.0, 25.0),
        CockpitFaction::Empire => (84.0, 27.0),
    }
}

/// The galaxy map's one transform for a faction's aperture at `display_scale`.
///
/// `FUN_00422ce0` has no wheel case and no right-button drag, so the map
/// neither zooms nor pans.
#[must_use]
pub fn galaxy_camera(
    viewport: (f32, f32, f32, f32),
    display_scale: f32,
    faction: CockpitFaction,
) -> CameraView {
    let (viewport_x, viewport_y, viewport_width, viewport_height) = viewport;
    let (aperture_x, aperture_y, _, _) = cockpit::galaxy_aperture(faction);
    let canvas_x = viewport_x - aperture_x * display_scale;
    let canvas_y = viewport_y - aperture_y * display_scale;
    let (content_x, content_y) = galaxy_content_origin(faction);
    CameraView {
        origin_x: canvas_x + (content_x + GALAXY_MAP_OFFSET.0) * display_scale,
        origin_y: canvas_y + (content_y + GALAXY_MAP_OFFSET.1) * display_scale,
        map_scale_x: GALAXY_MAP_SCALE.0 * display_scale,
        map_scale_y: GALAXY_MAP_SCALE.1 * display_scale,
        zoom: display_scale,
        logical_zoom: 1.0,
        display_scale,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
    }
}

/// All mutable UI state for the galaxy map view.
pub struct GalaxyMapState {
    pub selected_system: Option<SystemKey>,
    pub hovered_system: Option<SystemKey>,
    /// System activated by a primary release in the current frame.
    /// Consumers clear this by drawing the next map frame.
    pub activated_system: Option<SystemKey>,
    /// Whether the held primary button was pressed on the map, which then
    /// takes its release (the capture of the window that took the press).
    pub press_on_map: bool,
    /// True while an original-interface window or modal overlay owns the pointer.
    pub pointer_blocked: bool,
    /// True while a Mission order waits for its target (galaxy view mode 2).
    pub targeting: bool,
    pub show_sector_labels: bool,
    pub show_grid: bool,
    /// Cockpit viewport bounds for mouse input clamping.
    /// If set, mouse input outside this rect is ignored.
    pub viewport: Option<(f32, f32, f32, f32)>,
    /// Uniform scale of the original 640×480 strategic canvas.
    pub display_scale: f32,
    /// Each system's GID marker bitmap this frame, empty with the display
    /// off. The sector window copies it into its planet's bitmap
    /// (`FUN_0042d860`).
    pub gid_markers: std::collections::HashMap<SystemKey, u32>,
    /// The markers the galaxy view drew this frame and their screen centres,
    /// which a sector window's copy of the view shows shaded.
    pub drawn_gid_markers: Vec<(u32, [f32; 2])>,
}

/// Simulation views needed to reproduce the original GID filters.
///
/// The renderer reads these canonical systems directly. Deterministic browser
/// fixtures therefore exercise the same resource cache, metrics, marker
/// selection, and drawing path as a production campaign.
pub struct GidOverlayContext<'a> {
    pub movement: &'a MovementState,
    pub manufacturing: &'a ManufacturingState,
    pub economy: &'a EconomyState,
    pub missions: &'a MissionState,
    /// Systems in revolt (system `+0x88` bit 2).
    pub uprisings: &'a rebellion_core::uprising::UprisingState,
    /// The system displays `0x92` and `0x93` mark (the briefing's action
    /// selects it, `FUN_004c0fc0`).
    pub marked_system: Option<SystemKey>,
}

impl Default for GalaxyMapState {
    fn default() -> Self {
        Self {
            selected_system: None,
            hovered_system: None,
            activated_system: None,
            press_on_map: false,
            pointer_blocked: false,
            targeting: false,
            show_sector_labels: true,
            show_grid: false,
            viewport: None,
            display_scale: 1.0,
            gid_markers: std::collections::HashMap::new(),
            drawn_gid_markers: Vec::new(),
        }
    }
}

/// Paint the original STRATEGY galaxy starfield behind the command-center shell.
///
/// The 607x437 bitmap is not fitted to either faction aperture. The original
/// shells reveal it through every transparent opening, not only the map
/// aperture: the reference frames (squakenet 0129 and 0712, pravus 0121) show
/// starfield above, beside and below the map, and behind C-3PO through the
/// droid frame's transparent (palette index 0) pixels. It is painted across
/// the whole canvas. A dark fill remains underneath as a fail-closed fallback
/// when the original resource is unavailable. The scissor is cleared on return.
pub fn draw_galaxy_backdrop(
    layout: CockpitLayout,
    faction: CockpitFaction,
    cache: &mut BmpCache,
    gid_mode: GidMode,
) -> bool {
    let clip = galaxy_backdrop_clip(layout);
    set_cockpit_viewport_clip(Some(clip));
    draw_rectangle(
        clip.x,
        clip.y,
        clip.width,
        clip.height,
        Color::new(0.02, 0.02, 0.08, 1.0),
    );

    let Some(texture) =
        cache.get_macroquad_original(DllSource::Strategy, gid_backdrop_resource(gid_mode))
    else {
        set_cockpit_viewport_clip(None);
        return false;
    };
    let destination =
        galaxy_backdrop_destination(layout, faction, texture.width(), texture.height());
    draw_texture_ex(
        texture,
        destination.x,
        destination.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(destination.width, destination.height)),
            ..Default::default()
        },
    );
    set_cockpit_viewport_clip(None);
    true
}

/// The starfield's clip: the whole strategic canvas.
fn galaxy_backdrop_clip(layout: CockpitLayout) -> CockpitViewport {
    layout.canvas
}

/// True for a system in revolt. `UprisingState` records a real revolt
/// (system `+0x88` bit 2); `ControlKind::Uprising` is kept for fixtures.
#[must_use]
pub fn system_in_revolt(
    key: SystemKey,
    system: &System,
    uprisings: &rebellion_core::uprising::UprisingState,
) -> bool {
    matches!(system.control, ControlKind::Uprising(_)) || uprisings.is_uprising(key)
}

pub(crate) fn gid_backdrop_resource(gid_mode: GidMode) -> u32 {
    if gid_mode.is_active() {
        bmp_cache::resources::strategy::GALAXY_STARFIELD_DIM
    } else {
        bmp_cache::resources::strategy::GALAXY_STARFIELD_BRIGHT
    }
}

pub(crate) fn galaxy_backdrop_destination(
    layout: CockpitLayout,
    faction: CockpitFaction,
    source_width: f32,
    source_height: f32,
) -> CockpitViewport {
    let (content_x, content_y) = galaxy_content_origin(faction);
    CockpitViewport {
        x: layout.canvas.x + content_x * layout.scale,
        y: layout.canvas.y + content_y * layout.scale,
        width: source_width * layout.scale,
        height: source_height * layout.scale,
    }
}

/// Render the galaxy star map for one frame.
///
/// Handles all input and draws the recovered Popular Support GID baseline.
/// Returns a `CameraView` so later authentic overlays can share its transform.
pub fn draw_galaxy_map(
    world: &GameWorld,
    state: &mut GalaxyMapState,
    cache: &mut BmpCache,
    gid_mode: GidMode,
    faction: CockpitFaction,
    gid: &GidOverlayContext<'_>,
) -> CameraView {
    state.activated_system = None;

    let sw = screen_width();
    let sh = screen_height();
    let viewport = state.viewport.unwrap_or((0.0, 0.0, sw, sh));
    let (viewport_x, viewport_y, viewport_width, viewport_height) = viewport;
    let (mx, my) = mouse_position();
    let in_viewport = !state.pointer_blocked
        && mx >= viewport_x
        && mx < viewport_x + viewport_width
        && my >= viewport_y
        && my < viewport_y + viewport_height;

    let cam = galaxy_camera(viewport, state.display_scale, faction);

    // ── Find hovered system (reset each frame) ────────────────────────────────
    state.hovered_system = None;
    let hover_radius = cam.scale_pixels(8.5);

    // ── Draw systems ──────────────────────────────────────────────────────────
    // Detect the nearest system for input without adding replacement hover art.
    if in_viewport {
        let mut best_dist = hover_radius;
        for (key, system) in &world.systems {
            let (sx, sy) = cam.to_screen(f32::from(system.x), f32::from(system.y));
            if !cam.contains_with_margin(sx, sy, cam.scale_pixels(20.0)) {
                continue;
            }
            let dist = ((mx - sx).powi(2) + (my - sy).powi(2)).sqrt();
            if dist < best_dist {
                best_dist = dist;
                state.hovered_system = Some(key);
            }
        }
    }

    // Draw the native fixed-size marker bitmaps. Selection and hover
    // remain input states, but the replacement circles, glows, and labels are
    // deliberately absent from the parity surface.
    state.gid_markers.clear();
    state.drawn_gid_markers.clear();
    if gid_mode.is_active() {
        for (key, system) in &world.systems {
            let resource_id = gid_marker_for_system(world, key, system, faction, gid_mode, gid);
            state.gid_markers.insert(key, resource_id);
            let (sx, sy) = cam.to_screen(f32::from(system.x), f32::from(system.y));
            if !cam.contains_with_margin(sx, sy, cam.scale_pixels(20.0)) {
                continue;
            }
            draw_gid_marker(cache, resource_id, cam, sx, sy);
            state.drawn_gid_markers.push((resource_id, [sx, sy]));
        }
    }

    // ── Click to select ───────────────────────────────────────────────────────
    let available = map_press_selects(state, in_viewport);
    if let Some(system) = map_release_activates(
        &mut state.press_on_map,
        is_mouse_button_pressed(MouseButton::Left),
        is_mouse_button_released(MouseButton::Left),
        available,
        state.hovered_system,
    ) {
        state.selected_system = Some(system);
        state.activated_system = Some(system);
    }

    cam
}

/// Paint the active GID filter's caption, bold and in the faction text color,
/// centred across the top of the map aperture.
///
/// Drawn in the egui pass, after the macroquad map, with the proportional
/// font the day readout uses. Squakenet frames 0119 and 0712 show the caption
/// in a heavy proportional face, its capitals about 11 canvas pixels tall.
pub fn draw_gid_caption(
    ctx: &egui::Context,
    cam: CameraView,
    faction: CockpitFaction,
    mode: GidMode,
) {
    if !mode.is_active() {
        return;
    }
    let color = match faction {
        CockpitFaction::Alliance => egui::Color32::from_rgb(199, 41, 41),
        CockpitFaction::Empire => egui::Color32::from_rgb(41, 166, 51),
    };
    let centre = gid_caption_centre(cam);
    let painter = ctx.layer_painter(egui::LayerId::background());
    for offset in [0.0, cam.scale_pixels(GID_CAPTION_BOLD_OFFSET)] {
        painter.text(
            centre + egui::vec2(offset, 0.0),
            egui::Align2::CENTER_CENTER,
            mode.label(),
            egui::FontId::proportional(cam.scale_pixels(GID_CAPTION_FONT_HEIGHT)),
            color,
        );
    }
}

/// Screen point the caption's text box is centred on.
fn gid_caption_centre(cam: CameraView) -> egui::Pos2 {
    egui::pos2(
        cam.viewport_x + cam.viewport_width / 2.0,
        cam.viewport_y + cam.scale_pixels(GID_CAPTION_CENTRE_Y),
    )
}

/// GID caption font height, text-box centre below the aperture top, and
/// bold-pass offset, in canvas pixels.
const GID_CAPTION_FONT_HEIGHT: f32 = 15.0;
const GID_CAPTION_CENTRE_Y: f32 = 8.5;
const GID_CAPTION_BOLD_OFFSET: f32 = 0.6;

fn draw_gid_marker(
    cache: &mut BmpCache,
    resource_id: u32,
    cam: CameraView,
    center_x: f32,
    center_y: f32,
) {
    let Some(texture) = cache.get_macroquad_original(DllSource::Strategy, resource_id) else {
        return;
    };
    let width = texture.width() * cam.display_scale;
    let height = texture.height() * cam.display_scale;
    draw_texture_ex(
        texture,
        center_x - width / 2.0,
        center_y - height / 2.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width, height)),
            ..Default::default()
        },
    );
}

fn popular_support_marker_resource(system: &System, faction: CockpitFaction) -> u32 {
    let explored = system.exploration_status == ExplorationStatus::Explored;
    let popularity = match faction {
        CockpitFaction::Alliance => system.popularity_alliance,
        CockpitFaction::Empire => system.popularity_empire,
    };
    gid_marker_resource(system.control, explored, popularity)
}

fn gid_marker_for_system(
    world: &GameWorld,
    system_key: SystemKey,
    system: &System,
    faction: CockpitFaction,
    mode: GidMode,
    gid: &GidOverlayContext<'_>,
) -> u32 {
    let unexplored = bmp_cache::resources::strategy::GID_UNEXPLORED;
    if gid_shows_unexplored(mode, system) {
        return unexplored;
    }
    if mode == GidMode::PopularSupport {
        return popular_support_marker_resource(system, faction);
    }
    if let Some(marker) = gid_briefing_marker(world, system_key, system, faction, mode, gid) {
        return marker;
    }
    // An unpopulated system is marked like any other: FUN_0050b610 gives it
    // the no-side holder 3, whose table (+0x2b4) holds the neutral marks.
    // At an unexplored system Fleets Enroute counting no fleet takes side
    // 0, and FUN_0042b330 sets the item's flag 0x80 for side 0 (its jump
    // table at 0x42c39c), so FUN_0042cde0 draws the unexplored mark
    // (+0x2c4, 0x27ae). Active Personnel counting nobody takes the
    // holder's side even there.
    let known = system.exploration_status == ExplorationStatus::Explored;

    let player_is_alliance = faction == CockpitFaction::Alliance;
    if matches!(mode, GidMode::IdlePersonnel | GidMode::ActivePersonnel) {
        let count = gid_personnel(
            world,
            system_key,
            player_is_alliance,
            mode == GidMode::ActivePersonnel,
            gid.movement,
        );
        // FUN_00426e70(.., 1, 1, 1, 1, 1): one person is the largest mark.
        return gid_marker_for_side(
            count.colour(system.control.faction()),
            usize::from(count.total() > 0) * 3,
        );
    }
    if matches!(
        mode,
        GidMode::IdleFleets | GidMode::FleetsEnRoute | GidMode::BriefingFleets
    ) {
        let counts = gid_fleets(
            world,
            system_key,
            player_is_alliance,
            mode == GidMode::FleetsEnRoute,
            gid.movement,
        );
        if !known && counts == (0, 0) {
            return unexplored;
        }
        return gid_fleet_mark(counts, player_is_alliance, system.control.faction());
    }
    let value = gid_metric(world, system_key, system, player_is_alliance, mode, gid);
    let size = gid_metric_size(mode, value);
    gid_marker_resource_for_size(system.control, true, size)
}

/// The briefing's displays (`FUN_0042b330` cases `0x13..0x17`, `0x92`,
/// `0x93`; `ghidra/notes/advisor-briefing.md`), or `None` for the others.
///
/// - `0x13..0x16`: the largest mark (`0x52`) when the holder is the side
///   the display names and the support side (`+0x84` bits 2-3) is, or for
///   military control is not, that side; else the smallest; in the
///   holder's colour.
/// - `0x17`: the largest no-side mark; only unexplored systems reach it.
/// - `0x92`, `0x93`: the largest Empire / Alliance mark (value 7) for the
///   marked system, the unexplored mark for every other.
fn gid_briefing_marker(
    world: &GameWorld,
    system_key: SystemKey,
    system: &System,
    faction: CockpitFaction,
    mode: GidMode,
    gid: &GidOverlayContext<'_>,
) -> Option<u32> {
    use rebellion_core::dat::Faction;
    let (viewer, enemy) = match faction {
        CockpitFaction::Alliance => (Faction::Alliance, Faction::Empire),
        CockpitFaction::Empire => (Faction::Empire, Faction::Alliance),
    };
    let (side, loyal) = match mode {
        GidMode::OwnLoyalSystems => (viewer, true),
        GidMode::OwnMilitaryControl => (viewer, false),
        GidMode::EnemyLoyalSystems => (enemy, true),
        GidMode::EnemyMilitaryControl => (enemy, false),
        GidMode::UnexploredSystems => return Some(gid_marker_for_side(Some(Faction::Neutral), 3)),
        GidMode::MarkedImperialSystem | GidMode::MarkedAllianceSystem => {
            if gid.marked_system != Some(system_key) {
                return Some(bmp_cache::resources::strategy::GID_UNEXPLORED);
            }
            let side = if mode == GidMode::MarkedImperialSystem {
                Faction::Empire
            } else {
                Faction::Alliance
            };
            return Some(gid_marker_for_side(Some(side), 3));
        }
        _ => return None,
    };
    let holder = system.control.faction();
    let support =
        rebellion_core::economy::support_side(system, &world.gnprtb, world.difficulty_index);
    let shown = holder == Some(side) && (support == Some(side)) == loyal;
    Some(gid_marker_for_side(holder, usize::from(shown) * 3))
}

/// Fleets Enroute (`0x22`) and Active Personnel (`0x44`) count even an
/// unexplored system (`FUN_0042b330`'s `bVar3`).
fn gid_counts_unexplored(mode: GidMode) -> bool {
    matches!(mode, GidMode::FleetsEnRoute | GidMode::ActivePersonnel)
}

/// Whether a system shows the unexplored mark before its display counts
/// anything (`FUN_0042b330`): an unexplored system (`+0x88` bit 1,
/// `System_SetExplored`) does, and a destroyed one (`+0x50` bit 3) does
/// outside the fleet and personnel displays (`bVar4`); Fleets Enroute and
/// Active Personnel count either.
fn gid_shows_unexplored(mode: GidMode, system: &System) -> bool {
    // bVar4: 0x20..0x22, 0x43, 0x44, 0x92, 0x93.
    let fleets_or_personnel = matches!(
        mode,
        GidMode::BriefingFleets
            | GidMode::IdleFleets
            | GidMode::FleetsEnRoute
            | GidMode::IdlePersonnel
            | GidMode::ActivePersonnel
            | GidMode::MarkedImperialSystem
            | GidMode::MarkedAllianceSystem
    );
    let explored = system.exploration_status == ExplorationStatus::Explored;
    // Unexplored Systems (0x17) turns the gate round: an explored system
    // takes the unexplored mark, an unexplored one is counted.
    let unseen = if mode == GidMode::UnexploredSystems {
        explored
    } else {
        !explored
    };
    let hidden = unseen || (system.is_destroyed && !fleets_or_personnel);
    hidden && !gid_counts_unexplored(mode)
}

/// The fleets the Idle Fleets (`0x21`) and Fleets Enroute (`0x22`)
/// displays count at a system, the player's and the enemy's
/// (`FUN_004feea0(system, side, 1 / 2)`). port: an idle fleet orbits the
/// system; an en route one has a move order to it.
fn gid_fleets(
    world: &GameWorld,
    system: SystemKey,
    player_is_alliance: bool,
    en_route: bool,
    movement: &MovementState,
) -> (u32, u32) {
    let fleets: Vec<rebellion_core::ids::FleetKey> = if en_route {
        movement
            .orders()
            .iter()
            .filter(|(_, order)| order.destination == system)
            .map(|(&key, _)| key)
            .collect()
    } else {
        world
            .systems
            .get(system)
            .map(|place| place.fleets.clone())
            .unwrap_or_default()
    };
    let mut counts = (0, 0);
    for alliance in fleets
        .iter()
        .filter_map(|key| world.fleets.get(*key))
        .map(|fleet| fleet.is_alliance)
    {
        if alliance == player_is_alliance {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    counts
}

/// The fleet displays' mark (`FUN_0042b330` cases `0x20..0x22`): both
/// sides' fleets give the special art `0x27b0`; else the counting side's
/// colour, sized by its count through `(3, 2, 2, 1, 1)`; with none, the
/// holder's smallest mark.
fn gid_fleet_mark(
    (player, enemy): (u32, u32),
    player_is_alliance: bool,
    holder: Option<rebellion_core::dat::Faction>,
) -> u32 {
    use rebellion_core::dat::Faction;
    let side = |alliance: bool| {
        Some(if alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        })
    };
    match (player, enemy) {
        (0, 0) => gid_marker_for_side(holder, 0),
        (_, 0) => gid_marker_for_side(
            side(player_is_alliance),
            gid_metric_size(GidMode::IdleFleets, player),
        ),
        (0, _) => gid_marker_for_side(
            side(!player_is_alliance),
            gid_metric_size(GidMode::IdleFleets, enemy),
        ),
        _ => bmp_cache::resources::strategy::GID_BOTH_SIDES,
    }
}

/// The people the Idle and Active Personnel displays count at a system
/// (`FUN_0042b330` cases `0x43` / `0x44`): the player's characters and
/// special forces on the planet (`FUN_004f2640`, `FUN_005040c0`), and
/// every character aboard the fleets the system holds (`FUN_004ffe70`,
/// `FUN_00502db0`, `FUN_004f25c0`), the player's and the enemy's apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct GidPersonnel {
    player: u32,
    enemy: u32,
    player_is_alliance: bool,
}

impl GidPersonnel {
    fn total(self) -> u32 {
        self.player + self.enemy
    }

    /// The mark's colour: the player's side while it counts anyone, else
    /// the enemy's while it does, else the system's holder.
    fn colour(
        self,
        holder: Option<rebellion_core::dat::Faction>,
    ) -> Option<rebellion_core::dat::Faction> {
        use rebellion_core::dat::Faction;
        let side = |alliance: bool| {
            Some(if alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            })
        };
        if self.player > 0 {
            side(self.player_is_alliance)
        } else if self.enemy > 0 {
            side(!self.player_is_alliance)
        } else {
            holder
        }
    }
}

/// See [`GidPersonnel`]. A person is busy (Active) when it is not usable
/// (`+0x50` bit 0), on a mission (`+0x78` bit 7), captive (`+0xac` bit 0),
/// injured (`+0x94`) or holding a command (`+0x96`); a special force is
/// busy when not usable or on a mission. port: a killed character is not
/// usable; the port's fleets carry no special forces; the fleets a system
/// holds are those in orbit and those en route to it
/// ([`rebellion_core::movement::listed_fleets`]).
fn gid_personnel(
    world: &GameWorld,
    system: SystemKey,
    player_is_alliance: bool,
    active: bool,
    movement: &MovementState,
) -> GidPersonnel {
    let character_busy = |character: &rebellion_core::world::Character| {
        character.is_killed
            || character.on_mission
            || character.is_captive
            || character.injury != 0
            || character.command != rebellion_core::character_command::CommandKind::None
    };
    // FUN_004edc80: the character's side.
    let alliance_of = |character: &rebellion_core::world::Character| character.is_alliance;
    let mut count = GidPersonnel {
        player_is_alliance,
        ..GidPersonnel::default()
    };
    for character in world.characters.values() {
        if character.current_system == Some(system)
            && character.current_fleet.is_none()
            && (character.is_alliance || character.is_empire)
            && alliance_of(character) == player_is_alliance
            && character_busy(character) == active
        {
            count.player += 1;
        }
    }
    if let Some(place) = world.systems.get(system) {
        count.player += place
            .special_forces
            .iter()
            .filter_map(|&key| world.special_forces.get(key))
            .filter(|force| force.is_alliance == player_is_alliance && force.on_mission == active)
            .count() as u32;
    }
    for fleet in rebellion_core::movement::listed_fleets(movement, world, system) {
        let Some(value) = world.fleets.get(fleet) else {
            continue;
        };
        for character in value
            .characters
            .iter()
            .filter_map(|&key| world.characters.get(key))
        {
            if character_busy(character) != active {
                continue;
            }
            if alliance_of(character) == player_is_alliance {
                count.player += 1;
            } else {
                count.enemy += 1;
            }
        }
    }
    count
}

#[expect(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "Preserve upstream metric count conversion and keep the GID mode table together."
)]
fn gid_metric(
    world: &GameWorld,
    system_key: SystemKey,
    system: &System,
    player_is_alliance: bool,
    mode: GidMode,
    gid: &GidOverlayContext<'_>,
) -> u32 {
    use rebellion_core::dat::Faction;
    // FUN_0042b330 counts for the system's holder (`+0x24` bits 6..7,
    // `uVar16`), not the viewer: FUN_0052d370, FUN_0052cf40, FUN_0052c8c0,
    // FUN_0052c5a0, FUN_0052c270, FUN_005044f0, FUN_00503550,
    // FUN_0051c0d0, FUN_005276d0, FUN_005273d0 and FUN_00527150 all take
    // it. The idle yards (0x65..0x67) count only when the holder is the
    // viewer.
    let holder = system.control.faction();
    let held_by = |is_alliance: bool| {
        holder
            == Some(if is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            })
    };
    let viewer = if player_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    // Manual p. 48: an idle yard is one "that isn't already building
    // something"; each area builds on its own (FUN_00509670).
    let area_idle = |area| {
        gid.manufacturing
            .queue(system_key, area)
            .is_none_or(rebellion_core::manufacturing::ProductionQueue::is_empty)
    };
    match mode {
        // The briefing's side displays and marked systems take their mark in
        // gid_marker_for_system.
        GidMode::PopularSupport
        | GidMode::DisplayOff
        | GidMode::OwnLoyalSystems
        | GidMode::OwnMilitaryControl
        | GidMode::EnemyLoyalSystems
        | GidMode::EnemyMilitaryControl
        | GidMode::UnexploredSystems
        | GidMode::MarkedImperialSystem
        | GidMode::MarkedAllianceSystem => 0,
        // FUN_0042b330 case 0x91: the holder's regiments (FUN_005044f0,
        // families 0x10..0x13), armed defenses (FUN_00526e90, 0x22..0x27)
        // and squadrons (FUN_00503550).
        GidMode::AllDefenses => gid_all_defenses(world, system_key, system),
        GidMode::Uprisings => u32::from(system_in_revolt(system_key, system, gid.uprisings)),
        GidMode::IdleFleets | GidMode::FleetsEnRoute | GidMode::BriefingFleets => {
            gid_fleets(
                world,
                system_key,
                player_is_alliance,
                mode == GidMode::FleetsEnRoute,
                gid.movement,
            )
            .0
        }
        GidMode::IdlePersonnel | GidMode::ActivePersonnel => gid_personnel(
            world,
            system_key,
            player_is_alliance,
            mode == GidMode::ActivePersonnel,
            gid.movement,
        )
        .total(),
        GidMode::AvailableEnergy => {
            let used = gid
                .economy
                .per_system
                .get(&system_key)
                .map_or(0, |economy| economy.energy_allocated);
            u32::from(system.total_energy).saturating_sub(used)
        }
        GidMode::AvailableRawMaterial => {
            let used = gid
                .economy
                .per_system
                .get(&system_key)
                .map_or(0, |economy| economy.raw_material_allocated);
            u32::from(system.raw_materials).saturating_sub(used)
        }
        GidMode::Mines | GidMode::Refineries => system
            .production_facilities
            .iter()
            .filter(|key| {
                world
                    .production_facilities
                    .get(**key)
                    .is_some_and(|facility| {
                        Some(facility.side) == holder
                            && (facility.is_mine == (mode == GidMode::Mines))
                    })
            })
            .count() as u32,
        GidMode::Shipyards
        | GidMode::IdleShipyards
        | GidMode::TrainingFacilities
        | GidMode::IdleTrainingFacilities
        | GidMode::ConstructionYards
        | GidMode::IdleConstructionYards => system
            .manufacturing_facilities
            .iter()
            .filter(|key| {
                world
                    .manufacturing_facilities
                    .get(**key)
                    .is_some_and(|facility| {
                        let family = facility.class_dat_id.family();
                        let correct_type = match mode {
                            GidMode::Shipyards | GidMode::IdleShipyards => {
                                facility.is_shipyard || family == 0x28
                            }
                            GidMode::TrainingFacilities | GidMode::IdleTrainingFacilities => {
                                family == 0x29
                            }
                            GidMode::ConstructionYards | GidMode::IdleConstructionYards => {
                                matches!(family, 0x20 | 0x2a)
                            }
                            _ => false,
                        };
                        let idle_area = match mode {
                            GidMode::IdleShipyards => Some(ProductionArea::Shipyard),
                            GidMode::IdleTrainingFacilities => {
                                Some(ProductionArea::TrainingFacility)
                            }
                            GidMode::IdleConstructionYards => {
                                Some(ProductionArea::ConstructionYard)
                            }
                            _ => None,
                        };
                        Some(facility.side) == holder
                            && correct_type
                            && idle_area
                                .is_none_or(|area| holder == Some(viewer) && area_idle(area))
                    })
            })
            .count() as u32,
        GidMode::Troopers => system
            .ground_units
            .iter()
            .filter(|key| {
                world
                    .troops
                    .get(**key)
                    .is_some_and(|troop| held_by(troop.is_alliance))
            })
            .count() as u32,
        // hyp: the original's count is untraced; the side's squadrons aboard
        // its fleets and on the planet (FUN_005039d0) both count.
        GidMode::FighterSquadrons => {
            system
                .fleets
                .iter()
                .filter_map(|key| world.fleets.get(*key))
                .filter(|fleet| held_by(fleet.is_alliance))
                .flat_map(|fleet| fleet.fighters.iter())
                .map(|fighters| fighters.count)
                .sum::<u32>()
                + match holder {
                    Some(Faction::Alliance) => {
                        rebellion_core::squadrons::on_planet(world, system_key, true)
                    }
                    Some(Faction::Empire) => {
                        rebellion_core::squadrons::on_planet(world, system_key, false)
                    }
                    _ => 0,
                }
        }
        GidMode::DeathStarShields => system
            .defense_facilities
            .iter()
            .filter(|key| {
                world.defense_facilities.get(**key).is_some_and(|facility| {
                    Some(facility.side) == holder && facility.class_dat_id.index() == 4
                })
            })
            .count() as u32,
        GidMode::PlanetaryShieldGenerators => system
            .defense_facilities
            .iter()
            .filter(|key| {
                world.defense_facilities.get(**key).is_some_and(|facility| {
                    Some(facility.side) == holder && matches!(facility.class_dat_id.index(), 3 | 6)
                })
            })
            .count() as u32,
        GidMode::PlanetaryDefenseBatteries => system
            .defense_facilities
            .iter()
            .filter(|key| {
                world.defense_facilities.get(**key).is_some_and(|facility| {
                    Some(facility.side) == holder
                        && matches!(facility.class_dat_id.index(), 1 | 2 | 5)
                })
            })
            .count() as u32,
    }
}

/// The mark's size for a display's value (`FUN_0042b330` hands
/// `FUN_00426e70(value, side, largest, large_max, large_min, medium_max,
/// medium_min)` these thresholds per display).
fn gid_metric_size(mode: GidMode, value: u32) -> usize {
    match mode {
        // 0x12, 0x73: (1, 1, 1, 1, 1). 0x65..0x67: the value is 15 when
        // the player's system has an idle area of that kind, else 0, so
        // any idle yard is the largest mark.
        GidMode::Uprisings
        | GidMode::DeathStarShields
        | GidMode::IdleShipyards
        | GidMode::IdleTrainingFacilities
        | GidMode::IdleConstructionYards
        | GidMode::IdlePersonnel
        | GidMode::ActivePersonnel => usize::from(value > 0) * 3,
        // 0x21, 0x22, 0x52: (3, 2, 2, 1, 1).
        GidMode::IdleFleets | GidMode::FleetsEnRoute | GidMode::AvailableRawMaterial => match value
        {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        },
        // 0x62..0x64: (5, 4, 2, 1, 1).
        GidMode::Shipyards | GidMode::TrainingFacilities | GidMode::ConstructionYards => {
            match value {
                0 => 0,
                1 => 1,
                2..=4 => 2,
                _ => 3,
            }
        }
        // The rest: (6, 5, 3, 2, 1).
        _ => match value {
            0 => 0,
            1..=2 => 1,
            3..=5 => 2,
            _ => 3,
        },
    }
}

/// `0x91` All Defenses: the holder's regiments, armed defense facilities
/// (families `0x22..0x27`) and squadrons on the planet and aboard its
/// fleets at the system. hyp: the squadrons count as Fighter Squadrons'
/// do (`FUN_00503550` is that display's counter).
fn gid_all_defenses(world: &GameWorld, system_key: SystemKey, system: &System) -> u32 {
    use rebellion_core::dat::Faction;
    let Some(holder) = system
        .control
        .faction()
        .filter(|&side| side != Faction::Neutral)
    else {
        return 0;
    };
    let alliance = holder == Faction::Alliance;
    let regiments = system
        .ground_units
        .iter()
        .filter_map(|key| world.troops.get(*key))
        .filter(|troop| troop.is_alliance == alliance)
        .count();
    let defenses = system
        .defense_facilities
        .iter()
        .filter_map(|key| world.defense_facilities.get(*key))
        .filter(|facility| {
            facility.side == holder && (0x22..=0x27).contains(&facility.class_dat_id.family())
        })
        .count();
    let squadrons = system
        .fleets
        .iter()
        .filter_map(|key| world.fleets.get(*key))
        .filter(|fleet| fleet.is_alliance == alliance)
        .flat_map(|fleet| fleet.fighters.iter())
        .map(|fighters| fighters.count)
        .sum::<u32>()
        + rebellion_core::squadrons::on_planet(world, system_key, alliance);
    u32::try_from(regiments + defenses)
        .unwrap_or(u32::MAX)
        .saturating_add(squadrons)
}

fn gid_marker_resource_for_size(control: ControlKind, explored: bool, size: usize) -> u32 {
    if !explored {
        return bmp_cache::resources::strategy::GID_UNEXPLORED;
    }
    gid_marker_for_side(control.faction(), size)
}

/// The mark of `size` (0 smallest .. 3 largest) in a side's colour
/// (`FUN_00426e70`'s tables `+0x294`, `+0x2a4`, `+0x2b4`).
fn gid_marker_for_side(side: Option<rebellion_core::dat::Faction>, size: usize) -> u32 {
    use bmp_cache::resources::strategy;

    let size = size.min(3);
    match side {
        Some(rebellion_core::dat::Faction::Alliance) => [
            strategy::GID_ALLIANCE_SMALLEST,
            strategy::GID_ALLIANCE_MEDIUM,
            strategy::GID_ALLIANCE_LARGE,
            strategy::GID_ALLIANCE_LARGEST,
        ][size],
        Some(rebellion_core::dat::Faction::Empire) => [
            strategy::GID_EMPIRE_SMALLEST,
            strategy::GID_EMPIRE_MEDIUM,
            strategy::GID_EMPIRE_LARGE,
            strategy::GID_EMPIRE_LARGEST,
        ][size],
        Some(rebellion_core::dat::Faction::Neutral) | None => [
            strategy::GID_NEUTRAL_SMALLEST,
            strategy::GID_NEUTRAL_MEDIUM,
            strategy::GID_NEUTRAL_LARGE,
            strategy::GID_NEUTRAL_LARGEST,
        ][size],
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Popularity is clamped to 0..=1 and rounded to a 0..=100 marker percentage."
)]
fn gid_marker_resource(control: ControlKind, explored: bool, popularity: f32) -> u32 {
    use bmp_cache::resources::strategy;

    if !explored {
        return strategy::GID_UNEXPLORED;
    }

    let support = (popularity.clamp(0.0, 1.0) * 100.0).round() as u8;
    let size = match support {
        81..=u8::MAX => 3,
        60..=80 => 2,
        50..=59 => 1,
        _ => 0,
    };
    gid_marker_resource_for_size(control, explored, size)
}

/// Whether a left click on the map may select the system under it: the
/// pointer is on the map, not over a window, and no Mission order is
/// targeting. In targeting mode the galaxy view holds the capture, and
/// `FUN_00422ce0` has no `WM_LBUTTONDOWN` case, so the press does nothing;
/// the release targets.
fn map_press_selects(state: &GalaxyMapState, in_viewport: bool) -> bool {
    in_viewport && !state.targeting
}

/// The system a left click opens. `FUN_00422ce0` has no `WM_LBUTTONDOWN`
/// case in the galaxy view; its `WM_LBUTTONUP` opens the item under the
/// pointer (`+0x25c`, `FUN_00429ce0`). A press elsewhere keeps its release:
/// the release counts only after a press on the map, and only over it.
fn map_release_activates(
    press_on_map: &mut bool,
    pressed: bool,
    released: bool,
    available: bool,
    hovered: Option<SystemKey>,
) -> Option<SystemKey> {
    if pressed {
        *press_on_map = available;
    }
    if !released {
        return None;
    }
    let captured = std::mem::take(press_on_map);
    (captured && available).then_some(hovered).flatten()
}

// ---------------------------------------------------------------------------
// Galaxy map overlays (macroquad, called after draw_galaxy_map)
// ---------------------------------------------------------------------------

/// Coordinate transform: game-space → screen-space.
///
/// Mirrors the closure inside `draw_galaxy_map` so overlays align.
#[inline]
fn map_to_screen(dat_x: f32, dat_y: f32, cam: &CameraView) -> (f32, f32) {
    cam.to_screen(dat_x, dat_y)
}

/// True if the screen-space point is inside the visible map area.
#[inline]
fn in_map_viewport(sx: f32, sy: f32, cam: &CameraView) -> bool {
    cam.contains_with_margin(sx, sy, cam.scale_pixels(30.0))
}

/// Draw small facility indicator squares next to systems that have facilities.
///
/// Three icon types are shown in a horizontal row offset to the upper-right of
/// the system dot, each as a 5×5 pixel square scaled by zoom:
/// - Yellow  = production facilities (mines / refineries)
/// - Cyan    = manufacturing facilities (shipyards / training)
/// - Orange  = defense facilities
///
/// Icons are only drawn when `zoom >= 0.5` to avoid clutter at high zoom-out.
/// Scale with zoom up to a cap so they stay readable without dominating.
///
/// Pass the `CameraView` returned by `draw_galaxy_map`.
#[expect(
    clippy::cast_precision_loss,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
pub fn draw_facility_icons(world: &GameWorld, cam: &CameraView) {
    if cam.logical_zoom < 0.5 {
        return;
    }

    let icon_size = cam.zoomed_pixels(4.0, 2.5, 8.0);
    let gap = icon_size + cam.scale_pixels(1.5);

    for (_key, system) in &world.systems {
        let has_prod = !system.production_facilities.is_empty();
        let has_mfg = !system.manufacturing_facilities.is_empty();
        let has_def = !system.defense_facilities.is_empty();

        if !has_prod && !has_mfg && !has_def {
            continue;
        }

        let (sx, sy) = map_to_screen(f32::from(system.x), f32::from(system.y), cam);
        if !in_map_viewport(sx, sy, cam) {
            continue;
        }

        // Offset cluster to upper-right of the system dot so it doesn't
        // overlap the name label (which appears to the right) or the fleet
        // diamond (which appears directly above).
        let base_x = sx + 6.0 * cam.zoom;
        let base_y = sy - 8.0 * cam.zoom - icon_size;

        let mut col = 0;

        if has_prod {
            // Yellow: resource production
            let ix = base_x + col as f32 * gap;
            draw_rectangle(
                ix,
                base_y,
                icon_size,
                icon_size,
                Color::new(0.9, 0.8, 0.1, 0.85),
            );
            col += 1;
        }
        if has_mfg {
            // Cyan: manufacturing (shipyards)
            let ix = base_x + col as f32 * gap;
            draw_rectangle(
                ix,
                base_y,
                icon_size,
                icon_size,
                Color::new(0.2, 0.8, 0.9, 0.85),
            );
            col += 1;
        }
        if has_def {
            // Orange: defense installations
            let ix = base_x + col as f32 * gap;
            draw_rectangle(
                ix,
                base_y,
                icon_size,
                icon_size,
                Color::new(0.9, 0.5, 0.1, 0.85),
            );
        }
    }
}

/// Compute a convex hull (Graham scan) of a set of 2D screen-space points.
///
/// Returns the points in counter-clockwise order.  Returns an empty Vec if
/// fewer than 3 points are provided.
fn convex_hull(mut pts: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts;
    }

    // Sort by x then y.
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap()
            .then(a.1.partial_cmp(&b.1).unwrap())
    });
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() < 0.5);

    if pts.len() < 3 {
        return pts;
    }

    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| -> f32 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let n = pts.len();
    let mut hull: Vec<(f32, f32)> = Vec::with_capacity(2 * n);

    // Build lower hull.
    for &p in &pts {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }

    // Build upper hull.
    let lower_len = hull.len() + 1;
    for &p in pts.iter().rev() {
        while hull.len() >= lower_len && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
        {
            hull.pop();
        }
        hull.push(p);
    }

    hull.pop(); // Remove the last point (same as first).
    hull
}

/// Dim sector-specific color from the `SectorGroup`.
fn sector_boundary_color(group: rebellion_core::dat::SectorGroup) -> Color {
    match group {
        rebellion_core::dat::SectorGroup::Core => Color::new(0.7, 0.6, 0.2, 0.25), // warm gold — galactic core
        rebellion_core::dat::SectorGroup::RimInner => Color::new(0.3, 0.5, 0.7, 0.22), // cool blue — inner rim
        rebellion_core::dat::SectorGroup::RimOuter => Color::new(0.4, 0.3, 0.6, 0.20), // purple-grey — outer rim
    }
}

/// Draw translucent polygon outlines for each sector region.
///
/// Computes the convex hull of member systems' screen positions, expands it
/// outward by `padding` pixels, then draws the hull edges with `draw_line()`.
///
/// Toggle controlled via `GalaxyMapState::show_sector_labels` — if sector
/// labels are hidden, boundaries are also hidden.
///
/// Pass the `CameraView` returned by `draw_galaxy_map`.
#[expect(
    clippy::cast_precision_loss,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
pub fn draw_sector_boundaries(world: &GameWorld, cam: &CameraView, show: bool) {
    if !show {
        return;
    }

    let padding = 12.0 * cam.logical_zoom.clamp(0.5, 2.0) * cam.display_scale;

    for (_sec_key, sector) in &world.sectors {
        if sector.systems.len() < 2 {
            continue;
        }

        // Collect screen-space positions of all systems in this sector.
        let pts: Vec<(f32, f32)> = sector
            .systems
            .iter()
            .filter_map(|&sk| world.systems.get(sk))
            .map(|sys| map_to_screen(f32::from(sys.x), f32::from(sys.y), cam))
            .filter(|&(sx, sy)| in_map_viewport(sx, sy, cam))
            .collect();

        if pts.len() < 2 {
            continue;
        }

        let hull = if pts.len() == 2 {
            // Two-point degenerate case: just draw the line.
            pts.clone()
        } else {
            convex_hull(pts.clone())
        };

        if hull.len() < 2 {
            continue;
        }

        let color = sector_boundary_color(sector.group);

        // Expand each hull vertex outward from the centroid by `padding`.
        let cx = hull.iter().map(|p| p.0).sum::<f32>() / hull.len() as f32;
        let cy = hull.iter().map(|p| p.1).sum::<f32>() / hull.len() as f32;

        let expanded: Vec<(f32, f32)> = hull
            .iter()
            .map(|&(px, py)| {
                let dx = px - cx;
                let dy = py - cy;
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                (px + dx / len * padding, py + dy / len * padding)
            })
            .collect();

        // Draw edges.
        let n = expanded.len();
        for i in 0..n {
            let (x1, y1) = expanded[i];
            let (x2, y2) = expanded[(i + 1) % n];
            draw_line(x1, y1, x2, y2, cam.scale_pixels(1.0), color);
        }
    }
}

/// Draw a blockade indicator (pulsing red ring) around each blockaded system.
///
/// Renders a solid outer ring in danger-red at low alpha around the system dot
/// to visually flag active blockades without obscuring the system color.
///
/// Pass the `CameraView` returned by `draw_galaxy_map` and the current
/// `BlockadeState` from the simulation.
pub fn draw_blockade_indicators(world: &GameWorld, blockade: &BlockadeState, cam: &CameraView) {
    let blockaded = blockade.blockaded_systems();
    if blockaded.is_empty() {
        return;
    }

    let ring_radius = cam.zoomed_pixels(7.0, 5.0, 18.0);
    let ring_width = cam.scale_pixels(1.5);
    let ring_color = Color::new(0.9, 0.15, 0.15, 0.7);

    for &sys_key in blockaded {
        let Some(system) = world.systems.get(sys_key) else {
            continue;
        };

        let (sx, sy) = map_to_screen(f32::from(system.x), f32::from(system.y), cam);
        if !in_map_viewport(sx, sy, cam) {
            continue;
        }

        // Outer ring — solid dim red.
        draw_circle_lines(sx, sy, ring_radius, ring_width, ring_color);
        // Inner fill — very faint red tint over the system dot.
        draw_circle(
            sx,
            sy,
            ring_radius - ring_width,
            Color::new(0.9, 0.1, 0.1, 0.08),
        );
    }
}

/// Draw the system info side panel (right side).
///
/// Shows details for the currently selected system. Call inside
/// `egui_macroquad::ui(|ctx| { ... })`.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn draw_system_info_panel(ctx: &egui::Context, world: &GameWorld, state: &GalaxyMapState) {
    if let Some(sys_key) = state.selected_system {
        if let Some(system) = world.systems.get(sys_key) {
            egui::SidePanel::right("system_info")
                .min_width(280.0)
                .max_width(320.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // ── Header ───────────────────────────────────────────
                        let name_color = match system.control {
                            rebellion_core::world::ControlKind::Controlled(
                                rebellion_core::dat::Faction::Alliance,
                            ) => theme::ALLIANCE_BLUE,
                            rebellion_core::world::ControlKind::Controlled(
                                rebellion_core::dat::Faction::Empire,
                            ) => theme::EMPIRE_RED,
                            _ => theme::TEXT_PRIMARY,
                        };
                        ui.heading(egui::RichText::new(&system.name).color(name_color));

                        if system.is_headquarters {
                            ui.label(
                                egui::RichText::new("HEADQUARTERS")
                                    .color(theme::GOLD)
                                    .size(10.0)
                                    .strong(),
                            );
                        }
                        if system.is_destroyed {
                            ui.label(
                                egui::RichText::new("DESTROYED")
                                    .color(theme::DANGER_RED)
                                    .size(10.0)
                                    .strong(),
                            );
                        }

                        // Sector + region
                        if let Some(sector) = world.sectors.get(system.sector) {
                            let region = match sector.group {
                                rebellion_core::dat::SectorGroup::Core => "Core",
                                rebellion_core::dat::SectorGroup::RimInner => "Inner Rim",
                                rebellion_core::dat::SectorGroup::RimOuter => "Outer Rim",
                            };
                            ui.label(
                                egui::RichText::new(format!("{} — {}", sector.name, region))
                                    .color(theme::TEXT_SECONDARY)
                                    .size(11.0),
                            );
                        }

                        ui.separator();

                        // ── Popularity bars ──────────────────────────────────
                        ui.label(
                            egui::RichText::new("SUPPORT")
                                .color(theme::GOLD_DIM)
                                .size(10.0)
                                .strong(),
                        );

                        let alliance_pct = system.popularity_alliance;
                        let empire_pct = system.popularity_empire;

                        // Alliance bar
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Alliance")
                                    .color(theme::ALLIANCE_BLUE)
                                    .size(11.0),
                            );
                            let bar = egui::ProgressBar::new(alliance_pct)
                                .text(format!("{:.0}%", alliance_pct * 100.0))
                                .fill(theme::ALLIANCE_BLUE);
                            ui.add(bar);
                        });

                        // Empire bar
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Empire  ")
                                    .color(theme::EMPIRE_RED)
                                    .size(11.0),
                            );
                            let bar = egui::ProgressBar::new(empire_pct)
                                .text(format!("{:.0}%", empire_pct * 100.0))
                                .fill(theme::EMPIRE_RED);
                            ui.add(bar);
                        });

                        // Control status
                        let control_str = match system.control {
                            rebellion_core::world::ControlKind::Controlled(
                                rebellion_core::dat::Faction::Alliance,
                            ) => "Alliance Controlled",
                            rebellion_core::world::ControlKind::Controlled(
                                rebellion_core::dat::Faction::Empire,
                            ) => "Empire Controlled",
                            rebellion_core::world::ControlKind::Uncontrolled
                            | rebellion_core::world::ControlKind::Controlled(
                                rebellion_core::dat::Faction::Neutral,
                            ) => "Neutral",
                            rebellion_core::world::ControlKind::Contested => "Contested",
                            rebellion_core::world::ControlKind::Uprising(_) => "Uprising",
                        };
                        ui.label(
                            egui::RichText::new(control_str)
                                .color(theme::TEXT_SECONDARY)
                                .size(10.0),
                        );

                        ui.separator();

                        // ── Fleets ───────────────────────────────────────────
                        if !system.fleets.is_empty() {
                            ui.label(
                                egui::RichText::new("FLEETS")
                                    .color(theme::GOLD_DIM)
                                    .size(10.0)
                                    .strong(),
                            );
                            for &fleet_key in &system.fleets {
                                if let Some(fleet) = world.fleets.get(fleet_key) {
                                    let faction_color = if fleet.is_alliance {
                                        theme::ALLIANCE_BLUE
                                    } else {
                                        theme::EMPIRE_RED
                                    };
                                    let faction_tag = if fleet.is_alliance { "A" } else { "E" };

                                    let ship_count: u32 = fleet.ship_count();
                                    let fighter_count: u32 =
                                        fleet.fighters.iter().map(|e| e.count).sum();

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("[{faction_tag}]"))
                                                .color(faction_color)
                                                .size(11.0)
                                                .strong(),
                                        );

                                        let mut parts = Vec::new();
                                        if ship_count > 0 {
                                            parts.push(format!("{ship_count} ships"));
                                        }
                                        if fighter_count > 0 {
                                            parts.push(format!("{fighter_count} sqns"));
                                        }
                                        if fleet.has_death_star {
                                            parts.push("Death Star".to_string());
                                        }
                                        ui.label(
                                            egui::RichText::new(parts.join(", "))
                                                .color(theme::TEXT_PRIMARY)
                                                .size(11.0),
                                        );
                                    });

                                    // Ship class breakdown
                                    for (class_key, count) in fleet.ship_counts_by_class() {
                                        if let Some(class) =
                                            world.capital_ship_classes.get(class_key)
                                        {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "  {} ×{}",
                                                    class.name, count
                                                ))
                                                .color(theme::TEXT_SECONDARY)
                                                .size(10.0),
                                            );
                                        }
                                    }

                                    // Commander
                                    for &char_key in &fleet.characters {
                                        if let Some(c) = world.characters.get(char_key) {
                                            ui.label(
                                                egui::RichText::new(format!("  Cmd: {}", c.name))
                                                    .color(theme::GOLD_DIM)
                                                    .size(10.0),
                                            );
                                        }
                                    }
                                }
                            }
                            ui.add_space(4.0);
                        }

                        // ── Ground Units ─────────────────────────────────────
                        if !system.ground_units.is_empty() {
                            let alliance_troops = system
                                .ground_units
                                .iter()
                                .filter(|k| world.troops.get(**k).is_some_and(|t| t.is_alliance))
                                .count();
                            let empire_troops = system.ground_units.len() - alliance_troops;

                            ui.label(
                                egui::RichText::new(format!(
                                    "GROUND FORCES ({})",
                                    system.ground_units.len()
                                ))
                                .color(theme::GOLD_DIM)
                                .size(10.0)
                                .strong(),
                            );
                            if alliance_troops > 0 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Alliance: {alliance_troops} regiments"
                                    ))
                                    .color(theme::ALLIANCE_BLUE)
                                    .size(10.0),
                                );
                            }
                            if empire_troops > 0 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Empire: {empire_troops} regiments"
                                    ))
                                    .color(theme::EMPIRE_RED)
                                    .size(10.0),
                                );
                            }
                            ui.add_space(4.0);
                        }

                        // ── Facilities ────────────────────────────────────────
                        let total_fac = system.defense_facilities.len()
                            + system.manufacturing_facilities.len()
                            + system.production_facilities.len();
                        if total_fac > 0 {
                            ui.label(
                                egui::RichText::new(format!("FACILITIES ({total_fac})"))
                                    .color(theme::GOLD_DIM)
                                    .size(10.0)
                                    .strong(),
                            );

                            if !system.defense_facilities.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Defense: {}",
                                        system.defense_facilities.len()
                                    ))
                                    .color(theme::TEXT_SECONDARY)
                                    .size(10.0),
                                );
                            }
                            if !system.manufacturing_facilities.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Shipyards: {}",
                                        system.manufacturing_facilities.len()
                                    ))
                                    .color(theme::TEXT_SECONDARY)
                                    .size(10.0),
                                );
                            }
                            if !system.production_facilities.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Production: {}",
                                        system.production_facilities.len()
                                    ))
                                    .color(theme::TEXT_SECONDARY)
                                    .size(10.0),
                                );
                            }
                            ui.add_space(4.0);
                        }

                        // ── Coordinates (small, at bottom) ───────────────────
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("({}, {})", system.x, system.y))
                                .color(theme::TEXT_DISABLED)
                                .size(9.0),
                        );
                    });
                });
        }
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "These regression checks require exact copied values, endpoints, and pixel coordinates."
    )]
    fn galaxy_backdrop_follows_each_cockpits_content_origin_at_native_size() {
        // Source: FUN_00421c70 canvas origin, STRATEGY.DLL 607x437 galaxy backdrop.
        // FUN_00425d00 offsets the GID content by (21, 25) in the Alliance
        // cockpit and (84, 27) in the Imperial one. Registering the starfield
        // against squakenet frames 0129 and 0712 at 1280x960 measured it
        // (42, 51) and (39, 50) px from a (0, 0) and (63, 2) origin.
        let alliance = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let destination =
            galaxy_backdrop_destination(alliance, CockpitFaction::Alliance, 607.0, 437.0);
        assert_eq!(destination.x, 21.0);
        assert_eq!(destination.y, 25.0);
        assert_eq!(destination.width, 607.0);
        assert_eq!(destination.height, 437.0);
        assert_eq!(alliance.galaxy.x, 55.0);
        assert_eq!(alliance.galaxy.y, 40.0);

        let empire = CockpitState::new(CockpitFaction::Empire).layout_for(1280.0, 960.0);
        let destination = galaxy_backdrop_destination(empire, CockpitFaction::Empire, 607.0, 437.0);
        assert_eq!(destination.x, 168.0);
        assert_eq!(destination.y, 54.0);
        assert_eq!(destination.width, 1214.0);
        assert_eq!(destination.height, 874.0);
        assert_eq!(empire.galaxy.x, 240.0);
        assert_eq!(empire.galaxy.y, 80.0);
    }

    #[test]
    fn the_starfield_covers_the_whole_canvas_behind_the_shell_and_droids() {
        // Squakenet frames 0129 and 0712 show starfield through the shell
        // above, beside and below the map aperture, and frame 0129 shows it
        // behind C-3PO through the FUN_0042adb0 droid frame.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let layout = CockpitState::new(faction).layout_for(1280.0, 960.0);
            assert_eq!(galaxy_backdrop_clip(layout), layout.canvas);
        }
    }

    #[test]
    fn active_gid_uses_dim_backdrop_and_display_off_uses_bright() {
        // Source: STRATEGY.DLL starfield resources 901 (dim) and 902 (bright).
        assert_eq!(
            gid_backdrop_resource(GidMode::PopularSupport),
            bmp_cache::resources::strategy::GALAXY_STARFIELD_DIM
        );
        assert_eq!(
            gid_backdrop_resource(GidMode::DisplayOff),
            bmp_cache::resources::strategy::GALAXY_STARFIELD_BRIGHT
        );
    }

    #[test]
    fn popular_support_thresholds_select_native_marker_sizes() {
        // Source: STRATEGY.DLL GID marker resources (smallest/medium/large/largest per faction).
        use bmp_cache::resources::strategy;
        use rebellion_core::dat::Faction;

        for (support, expected) in [
            (0.49, strategy::GID_ALLIANCE_SMALLEST),
            (0.50, strategy::GID_ALLIANCE_MEDIUM),
            (0.60, strategy::GID_ALLIANCE_LARGE),
            (0.81, strategy::GID_ALLIANCE_LARGEST),
        ] {
            assert_eq!(
                gid_marker_resource(ControlKind::Controlled(Faction::Alliance), true, support),
                expected
            );
        }
        assert_eq!(
            gid_marker_resource(ControlKind::Controlled(Faction::Empire), true, 0.81),
            strategy::GID_EMPIRE_LARGEST
        );
        assert_eq!(
            gid_marker_resource(ControlKind::Uncontrolled, true, 0.60),
            strategy::GID_NEUTRAL_LARGE
        );
        assert_eq!(
            gid_marker_resource(ControlKind::Controlled(Faction::Alliance), false, 1.0),
            strategy::GID_UNEXPLORED
        );
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "These regression checks require exact copied values, endpoints, and pixel coordinates."
    )]
    fn camera_transform_includes_aperture_offset() {
        let camera = CameraView {
            origin_x: 340.0 - 450.0 * 2.0,
            origin_y: 217.5 - 470.0 * 2.0,
            map_scale_x: 2.0,
            map_scale_y: 2.0,
            zoom: 2.0,
            logical_zoom: 1.0,
            display_scale: 2.0,
            viewport_x: 100.0,
            viewport_y: 40.0,
            viewport_width: 480.0,
            viewport_height: 355.0,
        };

        assert_eq!(camera.to_screen(450.0, 470.0), (340.0, 217.5));
        assert!(camera.contains(100.0, 40.0));
        assert!(camera.contains(579.999, 394.999));
        assert!(!camera.contains(99.0, 40.0));
        assert!(!camera.contains(580.0, 394.0));
        assert!(!camera.contains(579.0, 395.0));
        assert!(camera.contains_with_margin(90.0, 30.0, 10.0));
        assert_eq!(camera.scale_pixels(8.0), 16.0);
        assert_eq!(camera.zoomed_pixels(4.0, 2.5, 8.0), 8.0);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "These regression checks require exact copied values, endpoints, and pixel coordinates."
    )]
    fn display_scale_does_not_change_logical_visibility_thresholds() {
        let baseline = CameraView {
            origin_x: 50.0,
            origin_y: 50.0,
            map_scale_x: 0.7,
            map_scale_y: 0.7,
            zoom: 0.7,
            logical_zoom: 0.7,
            display_scale: 1.0,
            viewport_x: 0.0,
            viewport_y: 0.0,
            viewport_width: 100.0,
            viewport_height: 100.0,
        };
        let widescreen = CameraView {
            zoom: 0.7 * 1.666_666_6,
            display_scale: 1.666_666_6,
            viewport_width: 166.666_66,
            viewport_height: 166.666_66,
            ..baseline
        };

        assert_eq!(baseline.logical_zoom, widescreen.logical_zoom);
        assert_eq!(baseline.logical_zoom > 0.8, widescreen.logical_zoom > 0.8);
        assert!(
            (widescreen.zoomed_pixels(4.0, 2.5, 8.0) / baseline.zoomed_pixels(4.0, 2.5, 8.0)
                - widescreen.display_scale)
                .abs()
                < 0.001
        );
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "The caption anchor is an exact sum of copied canvas offsets."
    )]
    fn the_gid_caption_centres_where_the_original_frames_draw_it() {
        // Squakenet frames 0119 (Alliance) and 0712 (Empire), resampled to
        // 1280x960: the caption's capitals span y 86..107, centred on canvas
        // x 298.75 and 361.25. Its text box centres 8.5 px below the
        // aperture top at canvas y 40.
        for (faction, centre_x) in [
            (CockpitFaction::Alliance, 298.75),
            (CockpitFaction::Empire, 361.25),
        ] {
            let (x, y, w, h) = cockpit::galaxy_aperture(faction);
            let camera = galaxy_camera((x * 2.0, y * 2.0, w * 2.0, h * 2.0), 2.0, faction);
            let centre = gid_caption_centre(camera);
            assert!(
                (centre.x / 2.0 - centre_x).abs() < 1.5,
                "{faction:?} {centre:?}"
            );
            assert_eq!(centre.y, 97.0);
        }
    }

    #[test]
    fn the_galaxy_map_matches_original_frame_registrations() {
        // Squakenet frame 0129 (Alliance cockpit, 854x480 capture of the
        // 640x480 canvas): the markers of system 0x109, DAT (346, 295), and
        // system 0x121, DAT (747, 490), were detected at these canvas points.
        let check = |faction, scale: f32, dat: (f32, f32), canvas: (f32, f32)| {
            let (x, y, w, h) = cockpit::galaxy_aperture(faction);
            let camera =
                galaxy_camera((x * scale, y * scale, w * scale, h * scale), scale, faction);
            let (sx, sy) = camera.to_screen(dat.0, dat.1);
            assert!(
                (sx - canvas.0 * scale).abs() < 2.0 * scale,
                "{sx} vs {}",
                canvas.0 * scale
            );
            assert!(
                (sy - canvas.1 * scale).abs() < 2.0 * scale,
                "{sy} vs {}",
                canvas.1 * scale
            );
        };
        for scale in [1.0, 2.0] {
            check(
                CockpitFaction::Alliance,
                scale,
                (346.0, 295.0),
                (234.7, 157.8),
            );
            check(
                CockpitFaction::Alliance,
                scale,
                (747.0, 490.0),
                (472.5, 241.6),
            );
            // Squakenet frame 0712 (Imperial cockpit): the same two markers.
            check(
                CockpitFaction::Empire,
                scale,
                (346.0, 295.0),
                (298.1, 160.0),
            );
            check(
                CockpitFaction::Empire,
                scale,
                (747.0, 490.0),
                (535.5, 244.5),
            );
        }
    }

    #[test]
    fn every_large_galaxy_system_is_drawn_in_both_cockpits() {
        // SYSTEMSD extremes across all 200 systems: x 118..791, y 103..839.
        // The southernmost Large-galaxy sector lands on the aperture's bottom
        // edge, so check against the map's 20 px draw-culling margin.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let (x, y, w, h) = cockpit::galaxy_aperture(faction);
            let camera = galaxy_camera((x, y, w, h), 1.0, faction);
            for (dx, dy) in [
                (118.0, 103.0),
                (791.0, 103.0),
                (118.0, 839.0),
                (791.0, 839.0),
            ] {
                let (sx, sy) = camera.to_screen(dx, dy);
                assert!(
                    camera.contains_with_margin(sx, sy, camera.scale_pixels(20.0)),
                    "{faction:?} ({dx}, {dy}) -> ({sx}, {sy})"
                );
            }
        }
    }

    #[test]
    fn a_system_opens_on_the_release_of_a_press_on_the_map() {
        // FUN_00422ce0: no WM_LBUTTONDOWN case; WM_LBUTTONUP opens the
        // hovered item.
        let hovered = Some(SystemKey::default());
        let mut held = false;
        // The press alone opens nothing; its release over the map does.
        assert_eq!(
            map_release_activates(&mut held, true, false, true, hovered),
            None
        );
        assert!(held);
        assert_eq!(
            map_release_activates(&mut held, false, false, true, hovered),
            None
        );
        assert_eq!(
            map_release_activates(&mut held, false, true, true, hovered),
            hovered
        );
        assert!(!held);
        // A click within one frame.
        assert_eq!(
            map_release_activates(&mut held, true, true, true, hovered),
            hovered
        );
        // A press in a window keeps its release, even over the map.
        assert_eq!(
            map_release_activates(&mut held, true, false, false, hovered),
            None
        );
        assert_eq!(
            map_release_activates(&mut held, false, true, true, hovered),
            None
        );
        // A release over a window opens nothing, nor one over empty space.
        map_release_activates(&mut held, true, false, true, hovered);
        assert_eq!(
            map_release_activates(&mut held, false, true, false, hovered),
            None
        );
        map_release_activates(&mut held, true, false, true, None);
        assert_eq!(
            map_release_activates(&mut held, false, true, true, None),
            None
        );
    }

    #[test]
    fn a_press_during_targeting_selects_no_system() {
        // FUN_00422ce0 mode 2: the captured press has no WM_LBUTTONDOWN case.
        let mut state = GalaxyMapState::default();
        assert!(map_press_selects(&state, true));
        // Off the map, or over a window, nothing is selected either.
        assert!(!map_press_selects(&state, false));

        state.targeting = true;
        assert!(!map_press_selects(&state, true));
    }

    #[test]
    fn gid_mode_marker_bands_cover_zero_threshold_and_high_values() {
        // FUN_0042b330 -> FUN_00426e70(value, side, largest, large_max,
        // large_min, medium_max, medium_min) per display.
        assert_eq!(gid_metric_size(GidMode::Uprisings, 0), 0);
        assert_eq!(gid_metric_size(GidMode::Uprisings, 1), 3);
        assert_eq!(gid_metric_size(GidMode::IdleFleets, 1), 1);
        assert_eq!(gid_metric_size(GidMode::IdleFleets, 2), 2);
        assert_eq!(gid_metric_size(GidMode::IdleFleets, 3), 3);
        assert_eq!(gid_metric_size(GidMode::Shipyards, 0), 0);
        assert_eq!(gid_metric_size(GidMode::Shipyards, 1), 1);
        assert_eq!(gid_metric_size(GidMode::Shipyards, 4), 2);
        assert_eq!(gid_metric_size(GidMode::Shipyards, 5), 3);
        assert_eq!(gid_metric_size(GidMode::AvailableEnergy, 2), 1);
        assert_eq!(gid_metric_size(GidMode::AvailableEnergy, 5), 2);
        assert_eq!(gid_metric_size(GidMode::AvailableEnergy, 6), 3);
        // 0x52: (3, 2, 2, 1, 1).
        assert_eq!(gid_metric_size(GidMode::AvailableRawMaterial, 0), 0);
        assert_eq!(gid_metric_size(GidMode::AvailableRawMaterial, 1), 1);
        assert_eq!(gid_metric_size(GidMode::AvailableRawMaterial, 2), 2);
        assert_eq!(gid_metric_size(GidMode::AvailableRawMaterial, 3), 3);
        // 0x65..0x67: 15 for an idle area, else 0; 0x43 / 0x44: (1, ..).
        for mode in [
            GidMode::IdleShipyards,
            GidMode::IdleTrainingFacilities,
            GidMode::IdleConstructionYards,
            GidMode::IdlePersonnel,
            GidMode::ActivePersonnel,
        ] {
            assert_eq!(gid_metric_size(mode, 0), 0);
            assert_eq!(gid_metric_size(mode, 1), 3);
        }
    }

    #[test]
    fn the_fleet_displays_mark_both_sides_and_the_counting_side() {
        // FUN_0042b330 0x21 / 0x22: both sides' fleets give 0x27b0 (STRATEGY
        // 10160); else the counting side's colour by (3, 2, 2, 1, 1); none,
        // the holder's smallest.
        use bmp_cache::resources::strategy as art;
        use rebellion_core::dat::Faction;
        assert_eq!(gid_fleet_mark((1, 1), true, None), art::GID_BOTH_SIDES);
        assert_eq!(
            gid_fleet_mark((3, 0), true, None),
            art::GID_ALLIANCE_LARGEST
        );
        assert_eq!(gid_fleet_mark((1, 0), false, None), art::GID_EMPIRE_MEDIUM);
        assert_eq!(
            gid_fleet_mark((0, 2), true, Some(Faction::Alliance)),
            art::GID_EMPIRE_LARGE
        );
        assert_eq!(
            gid_fleet_mark((0, 1), false, None),
            art::GID_ALLIANCE_MEDIUM
        );
        assert_eq!(
            gid_fleet_mark((0, 0), true, Some(Faction::Empire)),
            art::GID_EMPIRE_SMALLEST
        );
        assert_eq!(
            gid_fleet_mark((0, 0), true, None),
            art::GID_NEUTRAL_SMALLEST
        );
    }

    #[test]
    fn the_briefing_displays_follow_the_holder_and_the_support_side() {
        // FUN_0042b330 cases 0x13..0x17, 0x91..0x93
        // (ghidra/notes/advisor-briefing.md): 0x52 when the holder is the
        // named side and its support side (+0x84 bits 2-3, GNPRTB 7760) is
        // or is not that side; 0x17 marks the unexplored systems, the gate
        // turned round; 0x92 / 0x93 mark one system.
        use bmp_cache::resources::strategy as art;
        use rebellion_core::dat::{ExplorationStatus, Faction};
        let mut world = GameWorld::default();
        world.gnprtb =
            rebellion_core::world::GnprtbParams::new(vec![rebellion_core::world::GnprtbEntry {
                parameter_id: 7760,
                development: 60,
                alliance_sp_easy: 60,
                alliance_sp_medium: 60,
                alliance_sp_hard: 60,
                empire_sp_easy: 60,
                empire_sp_medium: 60,
                empire_sp_hard: 60,
                multiplayer: 60,
            }]);
        let mut place = |control, alliance_support: f32, explored| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
                name: "Naboo".into(),
                sector: Default::default(),
                x: 0,
                y: 0,
                exploration_status: if explored {
                    ExplorationStatus::Explored
                } else {
                    ExplorationStatus::Unexplored
                },
                popularity_alliance: alliance_support,
                popularity_empire: 1.0 - alliance_support,
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
            })
        };
        let loyal = place(ControlKind::Controlled(Faction::Alliance), 0.8, true);
        let held = place(ControlKind::Controlled(Faction::Alliance), 0.5, true);
        let imperial = place(ControlKind::Controlled(Faction::Empire), 0.2, true);
        let occupied = place(ControlKind::Controlled(Faction::Empire), 0.5, true);
        let unknown = place(ControlKind::Controlled(Faction::Empire), 0.2, false);
        let movement = rebellion_core::movement::MovementState::new();
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: Some(loyal),
        };
        let mark = |key, mode| {
            gid_marker_for_system(
                &world,
                key,
                &world.systems[key],
                CockpitFaction::Alliance,
                mode,
                &gid,
            )
        };
        assert_eq!(
            mark(loyal, GidMode::OwnLoyalSystems),
            art::GID_ALLIANCE_LARGEST
        );
        assert_eq!(
            mark(held, GidMode::OwnLoyalSystems),
            art::GID_ALLIANCE_SMALLEST
        );
        assert_eq!(
            mark(held, GidMode::OwnMilitaryControl),
            art::GID_ALLIANCE_LARGEST
        );
        assert_eq!(
            mark(loyal, GidMode::OwnMilitaryControl),
            art::GID_ALLIANCE_SMALLEST
        );
        assert_eq!(
            mark(imperial, GidMode::EnemyLoyalSystems),
            art::GID_EMPIRE_LARGEST
        );
        assert_eq!(
            mark(imperial, GidMode::EnemyMilitaryControl),
            art::GID_EMPIRE_SMALLEST
        );
        assert_eq!(
            mark(occupied, GidMode::EnemyMilitaryControl),
            art::GID_EMPIRE_LARGEST
        );
        assert_eq!(
            mark(loyal, GidMode::EnemyLoyalSystems),
            art::GID_ALLIANCE_SMALLEST
        );
        // 0x17: unexplored systems take the largest no-side mark, explored
        // ones the unexplored mark.
        assert_eq!(
            mark(unknown, GidMode::UnexploredSystems),
            art::GID_NEUTRAL_LARGEST
        );
        assert_eq!(mark(loyal, GidMode::UnexploredSystems), art::GID_UNEXPLORED);
        assert_eq!(mark(unknown, GidMode::OwnLoyalSystems), art::GID_UNEXPLORED);
        // 0x92 / 0x93: the marked system alone.
        assert_eq!(
            mark(loyal, GidMode::MarkedImperialSystem),
            art::GID_EMPIRE_LARGEST
        );
        assert_eq!(
            mark(loyal, GidMode::MarkedAllianceSystem),
            art::GID_ALLIANCE_LARGEST
        );
        assert_eq!(
            mark(held, GidMode::MarkedImperialSystem),
            art::GID_UNEXPLORED
        );
    }

    #[test]
    fn all_defenses_counts_the_holders_regiments_defenses_and_squadrons() {
        // FUN_0042b330 case 0x91: FUN_005044f0 + FUN_00526e90 (families
        // 0x22..0x27) + FUN_00503550, for the holder; (6, 5, 3, 2, 1).
        use rebellion_core::dat::Faction;
        use rebellion_core::world::{DefenseFacilityInstance, TroopUnit};
        let mut world = GameWorld::default();
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector: Default::default(),
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
            control: ControlKind::Controlled(Faction::Empire),
        });
        for is_alliance in [false, false, true] {
            let troop = world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
                is_alliance,
                regiment_strength: 100,
            });
            world.systems[key].ground_units.push(troop);
        }
        for (family, side) in [
            (0x22_u32, Faction::Empire),
            (0x27, Faction::Empire),
            (0x28, Faction::Empire),
            (0x23, Faction::Alliance),
        ] {
            let facility = world.defense_facilities.insert(DefenseFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(family << 24 | 1),
                side,
            });
            world.systems[key].defense_facilities.push(facility);
        }
        // Two Imperial regiments and two armed defenses (0x28 is not one,
        // nor is the Alliance's).
        assert_eq!(gid_all_defenses(&world, key, &world.systems[key]), 4);
        assert_eq!(gid_metric_size(GidMode::AllDefenses, 4), 2);
        // The holder's squadrons aboard its fleets and on the planet.
        let imperial_class = world.fighter_classes.insert(Default::default());
        let rebel_class = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_alliance: true,
                ..Default::default()
            });
        for (is_alliance, class, count) in [(false, imperial_class, 2), (true, rebel_class, 5)] {
            let fleet = world.fleets.insert(rebellion_core::world::Fleet {
                location: key,
                capital_ships: vec![],
                fighters: vec![rebellion_core::world::FighterEntry { class, count, carrier: None }],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[key].fleets.push(fleet);
            world.systems[key]
                .fighters
                .push(rebellion_core::world::FighterEntry { class, count: 1, carrier: None });
        }
        assert_eq!(
            gid_all_defenses(&world, key, &world.systems[key]),
            4 + 2 + 1
        );
        world.systems[key].control = ControlKind::Uncontrolled;
        assert_eq!(gid_all_defenses(&world, key, &world.systems[key]), 0);
    }

    #[test]
    fn the_fleet_displays_count_each_side_in_orbit_or_en_route() {
        use rebellion_core::world::Fleet;
        let mut world = GameWorld::default();
        let mut system = || {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
                name: "Naboo".into(),
                sector: Default::default(),
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
                control: ControlKind::Uncontrolled,
            })
        };
        let here = system();
        let there = system();
        let fleet = |world: &mut GameWorld, is_alliance, location| {
            world.fleets.insert(Fleet {
                location,
                capital_ships: vec![],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: Vec::new(),
            })
        };
        let rebels = fleet(&mut world, true, here);
        let empire = fleet(&mut world, false, here);
        let second = fleet(&mut world, false, here);
        world.systems[here].fleets = vec![rebels, empire, second];
        let traveller = fleet(&mut world, true, there);
        let mut movement = rebellion_core::movement::MovementState::new();
        assert!(movement.order(traveller, there, here, 5));
        assert_eq!(gid_fleets(&world, here, true, false, &movement), (1, 2));
        assert_eq!(gid_fleets(&world, here, false, false, &movement), (2, 1));
        assert_eq!(gid_fleets(&world, here, true, true, &movement), (1, 0));
        assert_eq!(gid_fleets(&world, there, true, true, &movement), (0, 0));
        // The displays draw from them: both sides in orbit, the player's
        // alone en route.
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        let mark = |mode| {
            gid_marker_for_system(
                &world,
                here,
                &world.systems[here],
                CockpitFaction::Empire,
                mode,
                &gid,
            )
        };
        use bmp_cache::resources::strategy as art;
        assert_eq!(mark(GidMode::IdleFleets), art::GID_BOTH_SIDES);
        assert_eq!(mark(GidMode::FleetsEnRoute), art::GID_ALLIANCE_MEDIUM);
        let metric = |mode| gid_metric(&world, here, &world.systems[here], false, mode, &gid);
        assert_eq!(metric(GidMode::IdleFleets), 2);
        assert_eq!(metric(GidMode::FleetsEnRoute), 0);
    }

    #[test]
    fn unexplored_destroyed_and_unpopulated_systems_show_what_the_original_shows() {
        // FUN_0042b330: an unexplored system shows the unexplored mark, and
        // a destroyed one does outside the fleet and personnel displays;
        // Fleets Enroute (0x22) and Active Personnel (0x44) count either.
        use rebellion_core::dat::ExplorationStatus;
        use rebellion_core::world::Fleet;
        let mut world = GameWorld::default();
        let mut system = |control| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
                name: "Naboo".into(),
                sector: Default::default(),
                x: 0,
                y: 0,
                exploration_status: ExplorationStatus::Unexplored,
                popularity_alliance: 0.9,
                popularity_empire: 0.1,
                is_populated: true,
                total_energy: 12,
                raw_materials: 11,
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
            })
        };
        let here = system(ControlKind::Controlled(
            rebellion_core::dat::Faction::Empire,
        ));
        let there = system(ControlKind::Uncontrolled);
        let empire = world.fleets.insert(Fleet {
            location: there,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let mut movement = rebellion_core::movement::MovementState::new();
        assert!(movement.order(empire, there, here, 5));
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        use bmp_cache::resources::strategy as art;
        let mark = |world: &GameWorld, key, mode| {
            gid_marker_for_system(
                world,
                key,
                &world.systems[key],
                CockpitFaction::Alliance,
                mode,
                &gid,
            )
        };
        // Unexplored: the enemy fleet en route still shows; nothing else.
        assert_eq!(
            mark(&world, here, GidMode::PopularSupport),
            art::GID_UNEXPLORED
        );
        assert_eq!(
            mark(&world, here, GidMode::AvailableEnergy),
            art::GID_UNEXPLORED
        );
        assert_eq!(mark(&world, here, GidMode::IdleFleets), art::GID_UNEXPLORED);
        assert_eq!(
            mark(&world, here, GidMode::FleetsEnRoute),
            art::GID_EMPIRE_MEDIUM
        );
        // Fleets Enroute counting no fleet there takes side 0, whose flag
        // 0x80 (jump table 0x42c39c) draws the unexplored mark.
        assert_eq!(
            mark(&world, there, GidMode::FleetsEnRoute),
            art::GID_UNEXPLORED
        );
        // Active Personnel counting nobody takes the holder's side, the
        // Empire here, at its smallest (FUN_0042b330 0x43/0x44).
        assert_eq!(
            mark(&world, here, GidMode::ActivePersonnel),
            art::GID_EMPIRE_SMALLEST
        );
        // Destroyed and explored: only the fleet and personnel displays count.
        let mut world = world;
        world.systems[here].exploration_status = ExplorationStatus::Explored;
        world.systems[here].is_destroyed = true;
        assert_eq!(
            mark(&world, here, GidMode::PopularSupport),
            art::GID_UNEXPLORED
        );
        assert_eq!(
            mark(&world, here, GidMode::AvailableEnergy),
            art::GID_UNEXPLORED
        );
        assert_eq!(
            mark(&world, here, GidMode::FleetsEnRoute),
            art::GID_EMPIRE_MEDIUM
        );
        assert_ne!(mark(&world, here, GidMode::IdleFleets), art::GID_UNEXPLORED);
        assert_ne!(
            mark(&world, here, GidMode::IdlePersonnel),
            art::GID_UNEXPLORED
        );
        world.systems[here].is_destroyed = false;
        assert_ne!(
            mark(&world, here, GidMode::AvailableEnergy),
            art::GID_UNEXPLORED
        );
        assert_ne!(
            mark(&world, here, GidMode::PopularSupport),
            art::GID_UNEXPLORED
        );
        // An explored, unpopulated system takes its holder's marks: with no
        // side, holder 3 (FUN_0050b610), the neutral table at +0x2b4
        // (FUN_00426ee0, 0x27aa..0x27ad).
        world.systems[there].exploration_status = ExplorationStatus::Explored;
        world.systems[there].is_populated = false;
        world.systems[there].total_energy = 0;
        world.systems[there].popularity_alliance = 0.0;
        assert_eq!(
            mark(&world, there, GidMode::AvailableEnergy),
            art::GID_NEUTRAL_SMALLEST
        );
        assert_eq!(
            mark(&world, there, GidMode::PopularSupport),
            art::GID_NEUTRAL_SMALLEST
        );
        assert_eq!(
            mark(&world, there, GidMode::FleetsEnRoute),
            art::GID_NEUTRAL_SMALLEST
        );
    }

    #[test]
    fn the_personnel_displays_count_the_planet_and_every_fleet_aboard() {
        // FUN_0042b330 0x43 / 0x44: the player's people on the planet, and
        // every character aboard the system's fleets; busy is on a
        // mission, captive, injured (+0x94) or commanding (+0x96).
        use rebellion_core::dat::Faction;
        use rebellion_core::world::{Character, Fleet};
        let mut world = GameWorld::default();
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector: Default::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.2,
            popularity_empire: 0.8,
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
            control: ControlKind::Controlled(Faction::Empire),
        });
        let movement = rebellion_core::movement::MovementState::new();
        let count = |world: &GameWorld, active| gid_personnel(world, key, true, active, &movement);
        // Nobody: the holder's colour.
        let none = count(&world, false);
        assert_eq!(none.total(), 0);
        assert_eq!(none.colour(Some(Faction::Empire)), Some(Faction::Empire));
        assert_eq!(
            none.colour(Some(Faction::Alliance)),
            Some(Faction::Alliance)
        );
        // An Imperial officer aboard an Imperial fleet: the enemy's count.
        let vader = world.characters.insert(Character {
            name: "Vader".into(),
            is_empire: true,
            ..Default::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: key,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![vader],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[key].fleets.push(fleet);
        world.characters[vader].current_fleet = Some(fleet);
        let enemy = count(&world, false);
        assert_eq!((enemy.player, enemy.enemy), (0, 1));
        assert_eq!(enemy.colour(Some(Faction::Empire)), Some(Faction::Empire));
        // The player's spy on the planet takes the colour.
        let spy = world.characters.insert(Character {
            name: "Bria".into(),
            is_alliance: true,
            current_system: Some(key),
            ..Default::default()
        });
        let both = count(&world, false);
        assert_eq!((both.player, both.enemy), (1, 1));
        assert_eq!(both.colour(Some(Faction::Empire)), Some(Faction::Alliance));
        // Injured or commanding is busy: Active, not Idle.
        world.characters[spy].injury = 5;
        assert_eq!(count(&world, false).player, 0);
        assert_eq!(count(&world, true).player, 1);
        world.characters[spy].injury = 0;
        world.characters[vader].command = rebellion_core::character_command::CommandKind::Admiral;
        assert_eq!(count(&world, true).enemy, 1);
        assert_eq!(count(&world, false).enemy, 0);
        // An Imperial officer on the planet is not counted for the Alliance.
        world.characters.insert(Character {
            name: "Tarkin".into(),
            is_empire: true,
            current_system: Some(key),
            ..Default::default()
        });
        assert_eq!(count(&world, false).total(), 1);
        // One person is the largest mark, in the counted side's colour.
        assert_eq!(
            gid_marker_for_side(both.colour(None), usize::from(both.total() > 0) * 3),
            bmp_cache::resources::strategy::GID_ALLIANCE_LARGEST
        );
        // Only the enemy counted: its colour over the holder's.
        assert_eq!(
            GidPersonnel {
                player: 0,
                enemy: 2,
                player_is_alliance: true
            }
            .colour(Some(Faction::Alliance)),
            Some(Faction::Empire)
        );
        // Captive or killed is busy too.
        world.characters[spy].is_captive = true;
        assert_eq!(count(&world, false).player, 0);
        assert_eq!(count(&world, true).player, 1);
        world.characters[spy].is_captive = false;
        world.characters[spy].is_killed = true;
        assert_eq!(count(&world, true).player, 1);
        world.characters[spy].is_killed = false;
        // The player's own officer aboard its fleet counts for the player.
        let mon = world.characters.insert(Character {
            name: "Mon".into(),
            is_alliance: true,
            ..Default::default()
        });
        let rebels = world.fleets.insert(Fleet {
            location: key,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![mon],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[key].fleets.push(rebels);
        world.characters[mon].current_fleet = Some(rebels);
        assert_eq!(count(&world, false).player, 2);
        // The player's special forces on the planet: idle, or on a mission.
        let force = world
            .special_forces
            .insert(rebellion_core::world::SpecialForceUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0001),
                is_alliance: true,
                skills: [0; 8],
                on_mission: false,
            });
        let theirs = world
            .special_forces
            .insert(rebellion_core::world::SpecialForceUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0005),
                is_alliance: false,
                skills: [0; 8],
                on_mission: false,
            });
        world.systems[key].special_forces = vec![force, theirs];
        assert_eq!(count(&world, false).player, 3);
        world.special_forces[force].on_mission = true;
        assert_eq!(count(&world, false).player, 2);
        assert_eq!(count(&world, true).player, 1);

        // The display draws through the same count, the largest mark or
        // the smallest, and gid_metric reads it too.
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        let mark = |world: &GameWorld, mode| {
            gid_marker_for_system(
                world,
                key,
                &world.systems[key],
                CockpitFaction::Alliance,
                mode,
                &gid,
            )
        };
        assert_eq!(
            mark(&world, GidMode::IdlePersonnel),
            bmp_cache::resources::strategy::GID_ALLIANCE_LARGEST
        );
        assert_eq!(
            gid_metric(
                &world,
                key,
                &world.systems[key],
                true,
                GidMode::ActivePersonnel,
                &gid
            ),
            2
        );
        assert_eq!(
            gid_metric(
                &world,
                key,
                &world.systems[key],
                true,
                GidMode::IdlePersonnel,
                &gid
            ),
            2
        );
        // A third idle person makes the counts differ.
        world.characters[vader].command = rebellion_core::character_command::CommandKind::None;
        assert_eq!(
            gid_metric(
                &world,
                key,
                &world.systems[key],
                true,
                GidMode::IdlePersonnel,
                &gid
            ),
            3
        );
        world.characters[vader].command = rebellion_core::character_command::CommandKind::Admiral;
        // Nobody idle: the holder's smallest mark.
        world.characters[mon].on_mission = true;
        world.characters[spy].on_mission = true;
        world.special_forces[theirs].on_mission = true;
        world.characters[vader].command = rebellion_core::character_command::CommandKind::Admiral;
        assert_eq!(
            mark(&world, GidMode::IdlePersonnel),
            bmp_cache::resources::strategy::GID_EMPIRE_SMALLEST
        );
    }

    #[test]
    fn the_uprisings_filter_marks_a_system_in_revolt() {
        // F-026: UprisingState, not ControlKind, records a real revolt.
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(rebellion_core::world::Sector {
            dat_id: rebellion_core::ids::DatId::new(0x9200_0000),
            name: "Sector".into(),
            group: rebellion_core::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.2,
            popularity_empire: 0.8,
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
            control: ControlKind::Controlled(rebellion_core::dat::Faction::Empire),
        });
        let movement = rebellion_core::movement::MovementState::new();
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let mut uprisings = rebellion_core::uprising::UprisingState::default();
        let metric = |uprisings: &rebellion_core::uprising::UprisingState, world: &GameWorld| {
            let gid = GidOverlayContext {
                movement: &movement,
                manufacturing: &manufacturing,
                economy: &economy,
                missions: &missions,
                uprisings,
                marked_system: None,
            };
            gid_metric(
                world,
                key,
                &world.systems[key],
                true,
                GidMode::Uprisings,
                &gid,
            )
        };
        assert_eq!(metric(&uprisings, &world), 0);
        uprisings.active_uprisings.insert(
            key,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        assert_eq!(metric(&uprisings, &world), 1);

        let quiet = rebellion_core::uprising::UprisingState::default();
        world.systems[key].control = ControlKind::Uprising(rebellion_core::dat::Faction::Empire);
        assert!(system_in_revolt(key, &world.systems[key], &quiet));
        assert_eq!(metric(&quiet, &world), 1);
    }
    // Manual p. 48: an idle training yard is one "that isn't already
    // building something"; a system's areas build on their own
    // (FUN_00509670), so its shipyard is idle while its troops train.
    #[test]
    fn an_idle_yard_is_one_whose_own_area_is_not_building() {
        let mut world = GameWorld::default();
        let mut facility = |family: u32| {
            world.manufacturing_facilities.insert(
                rebellion_core::world::ManufacturingFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new((family << 24) | 1),
                    side: rebellion_core::dat::Faction::Alliance,
                    is_shipyard: family == 0x28,
                },
            )
        };
        let yards = vec![facility(0x28), facility(0x29)];
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.8,
            popularity_empire: 0.2,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: yards,
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(rebellion_core::dat::Faction::Alliance),
        });
        let movement = rebellion_core::movement::MovementState::new();
        let mut manufacturing = ManufacturingState::new();
        manufacturing.enqueue(
            key,
            rebellion_core::manufacturing::QueueItem::new(
                rebellion_core::manufacturing::BuildableKind::Troop(
                    rebellion_core::ids::DatId::new(0x1000_0001),
                ),
                5,
            ),
        );
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        let metric = |mode| gid_metric(&world, key, &world.systems[key], true, mode, &gid);

        assert_eq!(metric(GidMode::IdleShipyards), 1);
        assert_eq!(metric(GidMode::IdleTrainingFacilities), 0);
        assert_eq!(metric(GidMode::TrainingFacilities), 1);
        // FUN_0042b330 0x62..0x64 and 0x71 count for the holder whoever
        // views; 0x65..0x67 only when the holder is the viewer.
        let enemy_view = |mode| gid_metric(&world, key, &world.systems[key], false, mode, &gid);
        assert_eq!(enemy_view(GidMode::Shipyards), 1);
        assert_eq!(enemy_view(GidMode::TrainingFacilities), 1);
        assert_eq!(enemy_view(GidMode::IdleShipyards), 0);
    }

    #[test]
    fn the_counting_displays_count_the_holder_s_units_whoever_views() {
        // FUN_0042b330 0x53, 0x54, 0x71, 0x73..0x75: FUN_0052d370,
        // FUN_0052cf40, FUN_005044f0, FUN_0051c0d0, FUN_005276d0,
        // FUN_005273d0 + FUN_00527150, each with the holder (uVar16).
        use rebellion_core::dat::Faction;
        use rebellion_core::world::{
            DefenseFacilityInstance, ProductionFacilityInstance, TroopUnit,
        };
        let mut world = GameWorld::default();
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.2,
            popularity_empire: 0.8,
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
            control: ControlKind::Controlled(Faction::Empire),
        });
        for is_alliance in [false, false, true] {
            let troop = world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
                is_alliance,
                regiment_strength: 100,
            });
            world.systems[key].ground_units.push(troop);
        }
        let mine = world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2b00_0001),
                side: Faction::Empire,
                is_mine: true,
            });
        world.systems[key].production_facilities.push(mine);
        for index in [1_u32, 3, 4] {
            let facility = world.defense_facilities.insert(DefenseFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2200_0000 | index),
                side: Faction::Empire,
            });
            world.systems[key].defense_facilities.push(facility);
        }
        let movement = rebellion_core::movement::MovementState::new();
        let manufacturing = ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        for viewer_is_alliance in [true, false] {
            let metric = |mode| {
                gid_metric(
                    &world,
                    key,
                    &world.systems[key],
                    viewer_is_alliance,
                    mode,
                    &gid,
                )
            };
            assert_eq!(metric(GidMode::Troopers), 2);
            assert_eq!(metric(GidMode::Mines), 1);
            assert_eq!(metric(GidMode::Refineries), 0);
            assert_eq!(metric(GidMode::DeathStarShields), 1);
            assert_eq!(metric(GidMode::PlanetaryShieldGenerators), 1);
            assert_eq!(metric(GidMode::PlanetaryDefenseBatteries), 1);
        }
        // An uncontrolled system's holder is side 0: nothing counts.
        world.systems[key].control = ControlKind::Uncontrolled;
        assert_eq!(
            gid_metric(
                &world,
                key,
                &world.systems[key],
                false,
                GidMode::Troopers,
                &gid
            ),
            0
        );
    }

    // FUN_0042b330 case 0x72: FUN_00503550(system, holder, 1), the holder
    // being the system's +0x24 bits 6..7 (uVar16), whoever views. The
    // squadrons on the planet (FUN_005039d0) join those aboard its fleets.
    // Until 2026-10-23 this test expected the viewer's squadrons, from an
    // untraced guess.
    #[test]
    fn the_squadron_display_counts_the_holder_s_squadrons_on_the_planet() {
        let mut world = GameWorld::default();
        let key = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Naboo".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.8,
            popularity_empire: 0.2,
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
            control: ControlKind::Controlled(rebellion_core::dat::Faction::Alliance),
        });
        for (alliance, count) in [(true, 2), (false, 5)] {
            let class = world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    is_alliance: alliance,
                    is_empire: !alliance,
                    ..Default::default()
                });
            rebellion_core::squadrons::add(&mut world.systems[key].fighters, class, count);
        }
        let movement = rebellion_core::movement::MovementState::new();
        let manufacturing = ManufacturingState::new();
        let economy = rebellion_core::economy::EconomyState::default();
        let missions = rebellion_core::missions::MissionState::new();
        let uprisings = rebellion_core::uprising::UprisingState::default();
        let gid = GidOverlayContext {
            movement: &movement,
            manufacturing: &manufacturing,
            economy: &economy,
            missions: &missions,
            uprisings: &uprisings,
            marked_system: None,
        };
        let metric = |alliance| {
            gid_metric(
                &world,
                key,
                &world.systems[key],
                alliance,
                GidMode::FighterSquadrons,
                &gid,
            )
        };
        assert_eq!(metric(true), 2);
        assert_eq!(metric(false), 2);
        world.systems[key].control = ControlKind::Controlled(rebellion_core::dat::Faction::Empire);
        let metric = |alliance| {
            gid_metric(
                &world,
                key,
                &world.systems[key],
                alliance,
                GidMode::FighterSquadrons,
                &gid,
            )
        };
        assert_eq!(metric(true), 5);
        assert_eq!(metric(false), 5);
    }
}
