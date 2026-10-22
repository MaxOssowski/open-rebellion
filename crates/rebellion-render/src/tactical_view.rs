//! Tactical combat view — 2D battlefield for player-involved space battles.
//!
//! When a battle involves the player's faction, instead of auto-resolving we
//! transition to `GameMode::TacticalCombat` and render this view.
//!
//! # Phases
//!
//! 1. **Placement**: Player positions capital ships in their deployment zone.
//!    Fighters auto-deploy around their carrier. AI side auto-places.
//! 2. **Combat**: Real-time phased combat with HUD (Task #8).
//! 3. **Results**: Summary screen showing losses (Task #10).
//!
//! # Integration
//!
//! ```ignore
//! // In main.rs combat resolution loop:
//! if battle_involves_player {
//!     tactical_state = TacticalState::new_battle(...);
//!     game_mode = GameMode::TacticalCombat;
//! } else {
//!     abstract_battle::resolve_space_battle(...); // auto-resolve AI vs AI
//! }
//! ```

use std::collections::{HashMap, HashSet};

use macroquad::prelude::*;
use rebellion_core::combat::{CombatSide as AutoResolveSide, SpaceCombatResult};
use rebellion_core::ids::{FleetKey, SystemKey};
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{resources, BmpCache, DllSource};
#[cfg(feature = "interface-test-fixtures")]
use crate::tactical_assets::TacticalLodView;
use crate::tactical_assets::{
    OriginalTacticalCamera, TacticalAssetRenderer, TacticalEffectRenderObject,
    TacticalFighterRenderObject, TacticalProjectileRenderObject, TacticalRenderObject,
    TacticalScreenProjection,
};
pub use rebellion_core::tactical::*;
use rebellion_core::tactical_resources::{TacticalCapitalShipResource, TacticalFighterResource};

/// Print the lines the battle simulation logged since the last call.
fn print_battle_diagnostics(session: &mut BattleSession) {
    for line in session.diagnostic_log.drain(..) {
        info!("{}", line);
    }
}

/// Browser-space position of a source retained-mode coordinate.
trait TacticalWorldRender {
    fn rendered(self) -> Vec3;
}

impl TacticalWorldRender for TacticalWorldPosition {
    fn rendered(self) -> Vec3 {
        // Direct3D retained mode is left-handed and the browser renderer is
        // right-handed. Preserve Y and reflect source Z.
        vec3(self.x, self.y, -self.z)
    }
}

/// HUD bitmaps of a tactical order.
trait TacticalOrderArt {
    #[must_use]
    fn maneuver_graphic(self) -> Option<u32>;
    #[must_use]
    fn mission_graphic(self, player_is_alliance: bool) -> Option<u32>;
}

impl TacticalOrderArt for TacticalOrder {
    fn maneuver_graphic(self) -> Option<u32> {
        use resources::tactical as art;
        match self {
            Self::LeftHook => Some(art::MANEUVER_HUD_LEFT_HOOK),
            Self::RightHook => Some(art::MANEUVER_HUD_RIGHT_HOOK),
            Self::Hammer => Some(art::MANEUVER_HUD_HAMMER),
            Self::Anvil => Some(art::MANEUVER_HUD_ANVIL),
            Self::HoldPosition => Some(art::MANEUVER_HUD_HOLD_POSITION),
            _ => None,
        }
    }

    fn mission_graphic(self, player_is_alliance: bool) -> Option<u32> {
        use resources::tactical as art;
        match (self, player_is_alliance) {
            (Self::AttackCapitalShips, true) => {
                Some(art::MISSIONS_HUD_ATTACK_CAPITAL_SHIPS_ALLIANCE)
            }
            (Self::AttackFighters, true) => Some(art::MISSIONS_HUD_ATTACK_FIGHTERS_ALLIANCE),
            (Self::Recover, true) => Some(art::MISSIONS_HUD_RECOVER_ALLIANCE),
            (Self::AttackDeathStar, true) => Some(art::MISSIONS_HUD_ATTACK_DEATH_STAR_ALLIANCE),
            (Self::AttackCapitalShips, false) => {
                Some(art::MISSIONS_HUD_ATTACK_CAPITAL_SHIPS_EMPIRE)
            }
            (Self::AttackFighters, false) => Some(art::MISSIONS_HUD_ATTACK_FIGHTERS_EMPIRE),
            (Self::Recover, false) => Some(art::MISSIONS_HUD_RECOVER_EMPIRE),
            _ => None,
        }
    }
}

/// Direct3DRM material colour of a projectile.
trait WeaponEffectColor {
    fn color(&self) -> Color;
}

impl WeaponEffectColor for WeaponEffect {
    fn color(&self) -> Color {
        match self.color_selector {
            1 => Color::new(0.0, 1.0, 0.0, 1.0),
            2 => Color::new(0.0, 0.0, 1.0, 1.0),
            _ => Color::new(1.0, 0.0, 0.0, 1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The original TACTICAL.DLL background is a complete 640×480 composition.
const TACTICAL_WIDTH: f32 = 640.0;
const TACTICAL_HEIGHT: f32 = 480.0;
/// Measured black aperture inside bitmap 1000. All battle primitives stay here.
const BATTLE_APERTURE: NativeRect = NativeRect::new(16.0, 28.0, 444.0, 439.0);

/// Default placement of the modeless 470x331 Battle Alert / Results window
/// on the original 640x480 command-center canvas. The original window is
/// draggable; the deterministic launcher keeps this centered position.
const BATTLE_RESULTS_WINDOW_X: f32 = 85.0;
const BATTLE_RESULTS_WINDOW_Y: f32 = 74.0;
const BATTLE_RESULTS_SCENE_X: f32 = 12.0;
const BATTLE_RESULTS_SCENE_Y: f32 = 13.0;
const BATTLE_RESULTS_RAIL_X: f32 = 412.0;
const BATTLE_ALERT_WINDOW_X: f32 = 65.0;
const BATTLE_ALERT_WINDOW_Y: f32 = 54.0;

/// Selected-capital-ship display panel.
const TACTICAL_SELECTED_SHIP_PANEL: u32 = 1302;

/// Native meter apertures inside TACTICAL 1302. The left meter is shield
/// strength; the right meter is hull integrity in the selected ship's faction
/// colour, as shown by both faction reference captures.
const TACTICAL_SELECTED_SHIELD_METER: NativeRect = NativeRect::new(514.0, 107.0, 40.0, 8.0);
const TACTICAL_SELECTED_HULL_METER: NativeRect = NativeRect::new(585.0, 107.0, 39.0, 8.0);
const TACTICAL_SELECTED_SHIP_ART_X: f32 = 507.0;
const TACTICAL_SELECTED_SHIP_ART_Y: f32 = 37.0;
const TACTICAL_SUBSYSTEM_X: [f32; 5] = [491.0, 518.0, 545.0, 572.0, 599.0];
const TACTICAL_SUBSYSTEM_Y: f32 = 130.0;

/// Default ship icon size when no sprite is available.
const DEFAULT_SHIP_SIZE: f32 = 40.0;

/// Fighter squadron icon size.
const FIGHTER_SIZE: f32 = 16.0;

/// `_DAT_0066d07c`, applied after each source maneuver rotation.
const ORIGINAL_MANEUVER_WAYPOINT_SCALE: f32 = 0.75;
/// `_DAT_0066d094` and `_DAT_0066d098` multiply pi by +/- one eighth.
const ORIGINAL_MANEUVER_ANGLE: f32 = std::f32::consts::PI / 8.0;
#[derive(Debug, Clone, Copy, PartialEq)]
struct NativeRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl NativeRect {
    const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

/// One uniformly scaled, letterboxed copy of the original tactical canvas.
#[derive(Debug, Clone, Copy)]
struct TacticalCanvas {
    x: f32,
    y: f32,
    scale: f32,
}

impl TacticalCanvas {
    fn new(width: f32, height: f32) -> Self {
        let scale = (width / TACTICAL_WIDTH)
            .min(height / TACTICAL_HEIGHT)
            .max(0.01);
        Self {
            x: (width - TACTICAL_WIDTH * scale) * 0.5,
            y: (height - TACTICAL_HEIGHT * scale) * 0.5,
            scale,
        }
    }

    fn point(self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.scale, self.y + y * self.scale)
    }

    fn logical_pointer(self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.x) / self.scale, (y - self.y) / self.scale)
    }

    fn aperture(self) -> NativeRect {
        let (x, y) = self.point(BATTLE_APERTURE.x, BATTLE_APERTURE.y);
        NativeRect::new(
            x,
            y,
            BATTLE_APERTURE.width * self.scale,
            BATTLE_APERTURE.height * self.scale,
        )
    }

    fn arena_transform(self, zoom: f32, camera_x: f32, camera_y: f32) -> (f32, f32, f32) {
        let aperture = self.aperture();
        let scale = (aperture.width / ARENA_WIDTH).min(aperture.height / ARENA_HEIGHT) * zoom;
        let x = aperture.x + aperture.width * 0.5 - (ARENA_WIDTH * 0.5 + camera_x) * scale;
        let y = aperture.y + aperture.height * 0.5 - (ARENA_HEIGHT * 0.5 + camera_y) * scale;
        (scale, x, y)
    }
}

// ---------------------------------------------------------------------------
// Battle phase
// ---------------------------------------------------------------------------

/// Original right-rail destination inside the Battle Results window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BattleResultTab {
    #[default]
    Summary,
    AllianceForces,
    ImperialForces,
    GoDirectlyTo,
}

/// Unit family selected inside an Alliance or Imperial force-detail tab.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BattleResultCategory {
    #[default]
    CapitalShips,
    Fighters,
    Troops,
    Personnel,
}

/// Original right-rail page inside the pre-battle Battle Alert window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum BattleAlertTab {
    #[default]
    Summary,
    AllianceForces,
    ImperialForces,
    System,
}

impl BattleAlertTab {
    const ALL: [Self; 4] = [
        Self::Summary,
        Self::AllianceForces,
        Self::ImperialForces,
        Self::System,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::AllianceForces => "alliance-forces",
            Self::ImperialForces => "imperial-forces",
            Self::System => "system-summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleAlertControl {
    Retreat,
    Simulate,
    TakeCommand,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TacticalCommandPanel {
    #[default]
    Display,
    Maneuvers {
        pending_order: TacticalOrder,
        pending_tactic: TacticalTactic,
    },
    Missions {
        pending_order: TacticalOrder,
    },
}

#[derive(Debug, Clone, Copy)]
struct TacticalCameraBookmark {
    source: OriginalTacticalCamera,
    camera_x: f32,
    camera_y: f32,
    zoom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalCameraFollow {
    TaskForce(u8),
    FighterGroup(u8),
}

/// Persistent state for the tactical combat view across frames.
pub struct TacticalState {
    /// The current battle session (None when not in combat).
    pub session: Option<BattleSession>,
    /// Ship being dragged during placement (index in session.ships).
    pub dragging_ship: Option<usize>,
    /// Mouse offset from ship center during drag.
    pub drag_offset: (f32, f32),
    /// Camera offset for panning the battlefield.
    pub camera_x: f32,
    pub camera_y: f32,
    /// Zoom level.
    pub zoom: f32,
    /// Independently visible navigation-point sets 1 through 4.
    navigation_sets_visible: [bool; 4],
    logged_navigation_scene: Option<[bool; 4]>,
    /// Default and player-memorized camera states for resources 1040–1043.
    default_camera: Option<TacticalCameraBookmark>,
    memorized_camera: Option<TacticalCameraBookmark>,
    /// Keyboard-driven task-force or fighter-group chase target.
    camera_follow: Option<TacticalCameraFollow>,
    /// Original Tactical Display faction-wireframe switches.
    pub highlight_alliance: bool,
    pub highlight_empire: bool,
    /// Active source tab in the post-battle Battle Results window.
    pub battle_result_tab: BattleResultTab,
    /// Active force family in the Alliance or Imperial result tab.
    pub battle_result_category: BattleResultCategory,
    /// The original strategic Battle Alert owns entry until the player takes
    /// command, simulates, or attempts to retreat.
    battle_alert_open: bool,
    battle_alert_tab: BattleAlertTab,
    battle_alert_pressed: Option<BattleAlertControl>,
    /// The strategic world already consumed the auto-resolve result. Closing
    /// the result window must not apply the same losses a second time.
    strategic_results_applied: bool,
    /// Whether the authentic Battle Options panel owns the right-hand housing.
    battle_options_open: bool,
    /// Captured Battle Options control. Dispatch occurs only on a matching release.
    battle_options_pressed: Option<TacticalBattleOptionsControl>,
    /// The native withdrawal confirmation replaces the right-hand options panel.
    withdraw_confirmation_open: bool,
    /// Captured confirmation control. Dispatch occurs only on a matching release.
    withdraw_confirmation_pressed: Option<TacticalWithdrawConfirmationControl>,
    /// The player handed command of their force to the tactical simulation.
    player_observing: bool,
    /// Captured press on the source Death Star superlaser control.
    death_star_laser_pressed: bool,
    /// The next valid right-click assigns the superlaser target.
    death_star_targeting: bool,
    command_panel: TacticalCommandPanel,
    warmed_font_sizes: HashSet<u16>,
    render_original_participants: bool,
    render_original_starfield: bool,
    render_original_planet: bool,
    render_pyrotechnics: bool,
    display_holocube: bool,
    asset_renderer: TacticalAssetRenderer,
    #[cfg(feature = "interface-test-fixtures")]
    suppress_mapped_capital_fallback: bool,
    #[cfg(feature = "interface-test-fixtures")]
    suppress_mapped_fighter_fallback: bool,
    #[cfg(feature = "interface-test-fixtures")]
    proof_resource_2560: bool,
    #[cfg(feature = "interface-test-fixtures")]
    proof_lod_follows_zoom: bool,
}

impl Default for TacticalState {
    fn default() -> Self {
        Self {
            session: None,
            dragging_ship: None,
            drag_offset: (0.0, 0.0),
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
            navigation_sets_visible: [false; 4],
            logged_navigation_scene: None,
            default_camera: None,
            memorized_camera: None,
            camera_follow: None,
            highlight_alliance: true,
            highlight_empire: true,
            battle_result_tab: BattleResultTab::Summary,
            battle_result_category: BattleResultCategory::CapitalShips,
            battle_alert_open: false,
            battle_alert_tab: BattleAlertTab::Summary,
            battle_alert_pressed: None,
            strategic_results_applied: false,
            battle_options_open: false,
            battle_options_pressed: None,
            withdraw_confirmation_open: false,
            withdraw_confirmation_pressed: None,
            player_observing: false,
            death_star_laser_pressed: false,
            death_star_targeting: false,
            command_panel: TacticalCommandPanel::Display,
            warmed_font_sizes: HashSet::new(),
            render_original_participants: true,
            render_original_starfield: true,
            render_original_planet: true,
            render_pyrotechnics: true,
            display_holocube: true,
            asset_renderer: TacticalAssetRenderer::default(),
            #[cfg(feature = "interface-test-fixtures")]
            suppress_mapped_capital_fallback: false,
            #[cfg(feature = "interface-test-fixtures")]
            suppress_mapped_fighter_fallback: false,
            #[cfg(feature = "interface-test-fixtures")]
            proof_resource_2560: false,
            #[cfg(feature = "interface-test-fixtures")]
            proof_lod_follows_zoom: false,
        }
    }
}

impl TacticalState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new battle — sets session and resets UI state.
    pub fn begin_battle(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        attacker: FleetKey,
        defender: FleetKey,
        player_is_attacker: bool,
        tick: u64,
        rng_seed: u32,
    ) {
        let session = BattleSession::new(
            world,
            system,
            attacker,
            defender,
            player_is_attacker,
            tick,
            rng_seed,
        );
        let player_is_empire = player_is_attacker != session.attacker_is_alliance;
        self.asset_renderer
            .set_palette_selector(session.system_picture_id);
        self.asset_renderer
            .enable_original_camera(player_is_empire, session.source_layout);
        self.session = Some(session);
        self.dragging_ship = None;
        self.drag_offset = (0.0, 0.0);
        self.camera_x = 0.0;
        self.camera_y = 0.0;
        self.zoom = 1.0;
        self.navigation_sets_visible = [false; 4];
        self.logged_navigation_scene = None;
        self.default_camera =
            self.asset_renderer
                .camera_state()
                .map(|source| TacticalCameraBookmark {
                    source,
                    camera_x: self.camera_x,
                    camera_y: self.camera_y,
                    zoom: self.zoom,
                });
        self.memorized_camera = None;
        self.camera_follow = None;
        self.highlight_alliance = true;
        self.highlight_empire = true;
        self.battle_result_tab = BattleResultTab::Summary;
        self.battle_result_category = BattleResultCategory::CapitalShips;
        self.battle_alert_open = true;
        self.battle_alert_tab = BattleAlertTab::Summary;
        self.battle_alert_pressed = None;
        self.strategic_results_applied = false;
        self.battle_options_open = false;
        self.battle_options_pressed = None;
        self.withdraw_confirmation_open = false;
        self.withdraw_confirmation_pressed = None;
        self.player_observing = false;
        self.death_star_laser_pressed = false;
        self.death_star_targeting = false;
        self.command_panel = TacticalCommandPanel::Display;
        self.render_original_participants = true;
        self.render_original_starfield = true;
        self.render_original_planet = true;
        self.render_pyrotechnics = true;
        self.display_holocube = true;
        #[cfg(feature = "interface-test-fixtures")]
        {
            self.suppress_mapped_capital_fallback = false;
            self.suppress_mapped_fighter_fallback = false;
            self.proof_resource_2560 = false;
            self.proof_lod_follows_zoom = false;
        }
    }

    /// Apply the five original Tactical Display options. The original Game
    /// Options screen disables these controls while a battle is in progress,
    /// so production callers update them only from outside tactical combat.
    pub fn set_display_options(&mut self, flags: [bool; 5]) {
        self.render_original_starfield = flags[0];
        self.render_original_planet = flags[1];
        self.render_pyrotechnics = flags[2];
        self.asset_renderer.set_high_detail(flags[3]);
        self.display_holocube = flags[4];
    }

    /// Test-only direct state for the source-backed empty-space presentation.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_empty_space_fixture(&mut self) {
        self.render_original_starfield = true;
        self.render_original_planet = false;
    }

    /// Enable the source-bound tactical LOD proof in isolated fixture builds.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn enable_resource_2560_proof(&mut self) {
        self.sync_proof_palette();
        self.proof_resource_2560 = true;
    }

    /// Set a deterministic source-coordinate LOD view for browser acceptance.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn set_tactical_lod_fixture(&mut self, view: TacticalLodView) {
        self.sync_proof_palette();
        self.proof_resource_2560 = true;
        self.proof_lod_follows_zoom = false;
        self.asset_renderer.set_view(view);
    }

    /// Drive all three source LOD slots through the authentic zoom controls.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn enable_tactical_lod_journey(&mut self) {
        self.sync_proof_palette();
        self.proof_resource_2560 = true;
        self.proof_lod_follows_zoom = true;
        self.zoom = 2.0;
        self.asset_renderer
            .set_view(TacticalLodView::from_fixture_zoom(self.zoom));
    }

    /// Enable the source-traced camera for isolated browser acceptance.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn enable_original_camera_proof(&mut self, player_is_empire: bool) {
        self.sync_proof_palette();
        self.proof_resource_2560 = true;
        self.proof_lod_follows_zoom = false;
        let source_layout = self.session.as_ref().map_or_else(
            || OriginalTacticalLayout::from_active_counts(0, 0),
            |session| session.source_layout,
        );
        self.asset_renderer
            .enable_original_camera(player_is_empire, source_layout);
    }

    #[cfg(feature = "interface-test-fixtures")]
    fn sync_proof_palette(&mut self) {
        let selector = self
            .session
            .as_ref()
            .map_or(1, |session| session.system_picture_id);
        self.asset_renderer.set_palette_selector(selector);
    }

    /// Preserve the historical proof and negative-control fixtures without
    /// submitting live participant families alongside the isolated object.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn disable_original_participant_rendering(&mut self) {
        self.render_original_participants = false;
        self.render_original_planet = false;
        self.asset_renderer.disable_original_camera();
    }

    /// Remove mapped participant fallbacks in the browser-only framebuffer control.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn suppress_mapped_capital_fallback(&mut self) {
        self.suppress_mapped_capital_fallback = true;
        self.suppress_mapped_fighter_fallback = true;
        // Keep the common system backdrop present so the paired framebuffer
        // remains a participant-only negative control.
        self.render_original_planet = true;
    }

    /// Center the source camera on the player's first production fighter group
    /// for deterministic close/far browser evidence. This never enters a
    /// production build; normal play must reach the same state through the
    /// original fighter-group controls.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn focus_player_fighter_for_fixture(&mut self) {
        let target = self.session.as_ref().and_then(|session| {
            session
                .fighters
                .iter()
                .enumerate()
                .find(|(_, fighter)| {
                    fighter.alive && fighter.is_attacker == session.player_is_attacker
                })
                .map(|(index, fighter)| {
                    (
                        u32::try_from(index)
                            .unwrap_or(u32::MAX)
                            .saturating_add(1001),
                        fighter.source_position.rendered(),
                    )
                })
        });
        if let Some((object_id, position)) = target {
            self.asset_renderer.focus_target(object_id, position);
        }
    }

    /// Populate every source group slot for deterministic browser coverage.
    /// Production battles retain the executable's proven initial states.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_group_presentation_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut task_force = 0_u8;
        for ship in &mut session.ships {
            if ship.alive && ship.is_attacker == session.player_is_attacker {
                ship.task_force = task_force.min(7);
                task_force = task_force.saturating_add(1);
            }
        }
        let mut fighter_group = 0_u8;
        for fighter in &mut session.fighters {
            if fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker == session.player_is_attacker
            {
                fighter.fighter_group = fighter_group.min(3);
                fighter_group = fighter_group.saturating_add(1);
            }
        }
    }

    /// Put three live player capitals in one task force so the recovered
    /// previous/next controls can prove their exact cyclic selection contract.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_selected_navigation_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut selected = None;
        for (index, ship) in session.ships.iter_mut().enumerate() {
            ship.selected = false;
            if ship.alive && ship.is_attacker == session.player_is_attacker {
                ship.task_force = 0;
                selected.get_or_insert(index);
            }
        }
        for fighter in &mut session.fighters {
            fighter.selected = false;
        }
        if let Some(index) = selected {
            session.ships[index].selected = true;
            session.selected_ship = Some(index);
            session.selected_fighter_group = None;
        }
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Select one player capital and make the first player fighter group
    /// addressable through the authentic command panels. Production play
    /// reaches the same state through the task-force and fighter controls.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_command_assignment_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(ship_index) = session
            .ships
            .iter()
            .position(|ship| ship.alive && ship.is_attacker == session.player_is_attacker)
        else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
            if ship.alive && ship.is_attacker == session.player_is_attacker {
                ship.fighter_capacity = 1;
            }
        }
        for fighter in &mut session.fighters {
            fighter.selected = false;
            if fighter.alive && fighter.is_attacker == session.player_is_attacker {
                fighter.fighter_group = 0;
            }
        }
        session.ships[ship_index].selected = true;
        session.selected_ship = Some(ship_index);
        session.selected_fighter_group = None;
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Keep one player fighter group addressable while preserving the
    /// production Death Star manager object for deterministic browser proof.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_death_star_presentation_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        for fighter in &mut session.fighters {
            fighter.selected = false;
            if fighter.alive && fighter.is_attacker == session.player_is_attacker {
                fighter.fighter_group = 0;
            }
        }
        session.selected_fighter_group = None;
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;

        // Exercise the other production branch of `FUN_005d0340` when the
        // local player owns the Imperial station. The ordinary Alliance
        // journey reaches the RGBG branch through the authentic mission
        // control; this fixture-only setup lets the browser prove the exact
        // station-under-attack caller without inventing an Imperial command.
        if session.player_voice_faction() == TacticalVoiceFaction::Empire {
            if let Some(index) = session.fighters.iter().position(|fighter| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.identity.is_alliance
                    && fighter.is_attacker != session.player_is_attacker
            }) {
                session.fighters[index].fighter_group = 0;
                session.fighters[index].order = TacticalOrder::AttackDeathStar;
                begin_original_attack_order(session, &[], &[index], TacticalOrder::AttackDeathStar);
            }
        }
    }

    /// Fully charge the production Death Star manager for the deterministic
    /// superlaser control and right-click target journey.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_death_star_laser_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Some(death_star) = session.death_star.as_mut() {
            death_star.laser_charge = ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE;
            death_star.action_committed = false;
        }
        if player_death_star(session).is_some() {
            if let Some(target) = session
                .ships
                .iter_mut()
                .find(|ship| ship.alive && ship.is_attacker != session.player_is_attacker)
            {
                target.source_position.x = 0.0;
                target.source_position.y = 40.0;
                target.source_position.z = session.source_layout.outer_positive_z;
            }
        }
        session.death_star_beam = None;
        session.paused = true;
        self.death_star_laser_pressed = false;
        self.death_star_targeting = false;
    }

    /// Drive the recovered state-5 producer to a deterministic source result
    /// so the browser can prove both native resolution and app-level 201/202
    /// routing without waiting two wall-clock minutes.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_trench_run_outcome_fixture(&mut self, outcome: TacticalTrenchRunOutcome) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(index) = session.fighters.iter().position(|fighter| {
            fighter.alive
                && fighter.squad_count > 0
                && fighter.identity.is_alliance
                && session
                    .death_star
                    .is_some_and(|death_star| fighter.is_attacker != death_star.is_attacker)
        }) else {
            return;
        };
        session.fighters[index].fighter_group = 0;
        session.fighters[index].order = TacticalOrder::AttackDeathStar;
        session.fighters[index].maneuverability = 9;
        match outcome {
            TacticalTrenchRunOutcome::Success => {
                // Seed 15 yields a maneuver draw of 8 and a success draw of 6.
                session.trench_run_success_rating = 9;
                session.tactical_rng = OriginalTacticalRng::new(15);
            }
            TacticalTrenchRunOutcome::Failure => {
                // Seed 1 yields a maneuver draw of 2 and a success draw of 68.
                session.trench_run_success_rating = 1;
                session.tactical_rng = OriginalTacticalRng::new(1);
            }
        };
        if session.trench_run_remaining_ms.is_none() {
            begin_original_attack_order(session, &[], &[index], TacticalOrder::AttackDeathStar);
        }
        session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
    }

    /// Populate each post-battle condition column and enter the production
    /// Battle Results phase without running an entire deterministic battle.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_battle_results_presentation_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        for alliance in [true, false] {
            let mut ships = session
                .ships
                .iter_mut()
                .filter(|ship| ship.identity.is_alliance == alliance);
            if let Some(ship) = ships.next() {
                ship.alive = true;
                ship.hull_current = ship.hull_max;
                ship.subsystem_damage = TacticalSubsystemDamage::default();
                ship.refresh_subsystem_condition();
            }
            if let Some(ship) = ships.next() {
                ship.alive = true;
                ship.hull_current = (ship.hull_max / 2).max(1);
                ship.subsystem_damage.engines = 1;
                ship.refresh_subsystem_condition();
            }
            if let Some(ship) = ships.next() {
                ship.alive = false;
                ship.hull_current = 0;
                ship.shield = 0;
                ship.refresh_subsystem_condition();
            }

            let mut fighters = session
                .fighters
                .iter_mut()
                .filter(|fighter| fighter.identity.is_alliance == alliance);
            if let Some(fighter) = fighters.next() {
                fighter.alive = true;
                fighter.hull_current = fighter.hull_max;
                fighter.squad_count = 1;
            }
            if let Some(fighter) = fighters.next() {
                fighter.alive = true;
                fighter.hull_current = (fighter.hull_max * 0.5).max(1.0);
                fighter.squad_count = 1;
            }
            if let Some(fighter) = fighters.next() {
                fighter.alive = false;
                fighter.hull_current = 0.0;
                fighter.squad_count = 0;
            }
        }
        session.phase = BattlePhase::Results;
        session.winner = Some(if session.player_is_attacker {
            CombatWinner::Attacker
        } else {
            CombatWinner::Defender
        });
        session.paused = true;
        session.selected_ship = None;
        session.selected_fighter_group = None;
        self.battle_result_tab = BattleResultTab::Summary;
        self.battle_result_category = BattleResultCategory::CapitalShips;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Keep one player ship withdrawal-capable while disabling another one's
    /// hyperdrive so the production confirmation route emits the source
    /// faction warning and leaves only the operational ship eligible.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_withdrawal_warning_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut player_ships = session
            .ships
            .iter_mut()
            .filter(|ship| ship.alive && ship.is_attacker == session.player_is_attacker);
        let Some(operational) = player_ships.next() else {
            return;
        };
        operational.subsystem_damage.hyperdrive = 0;
        operational.refresh_subsystem_condition();
        let Some(stranded) = player_ships.next() else {
            return;
        };
        stranded.subsystem_damage.hyperdrive = stranded.subsystem_capacity.hyperdrive;
        stranded.refresh_subsystem_condition();
    }

    /// Commit one recovered capital maneuver and one fighter recovery order
    /// through the production assignment and executor path.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_command_execution_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(ship_index) = session.ships.iter().position(|ship| {
            ship.alive
                && ship.is_attacker == session.player_is_attacker
                && ship.fighter_capacity > 0
        }) else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
        }
        for fighter in &mut session.fighters {
            fighter.selected = false;
            if fighter.alive && fighter.is_attacker == session.player_is_attacker {
                fighter.fighter_group = 0;
            }
        }
        let forward_z = if session.ships[ship_index].identity.is_alliance {
            1.0
        } else {
            -1.0
        };
        session.ships[ship_index].source_forward = TacticalWorldVector {
            x: 0.0,
            y: 0.0,
            z: forward_z,
        };
        session.ships[ship_index].selected = true;
        session.selected_ship = Some(ship_index);
        session.selected_fighter_group = None;
        assign_selected_command(
            session,
            TacticalOrder::LeftHook,
            Some(TacticalTactic::Surround),
        );

        if let Some(hold_index) = session.ships.iter().enumerate().find_map(|(index, ship)| {
            (index != ship_index && ship.alive && ship.is_attacker == session.player_is_attacker)
                .then_some(index)
        }) {
            session.ships[ship_index].selected = false;
            let hold_ship = &mut session.ships[hold_index];
            hold_ship.selected = true;
            hold_ship.source_desired_forward = hold_ship.source_forward;
            hold_ship.source_velocity = hold_ship.source_forward;
            hold_ship.source_waypoint = Some(hold_ship.source_position);
            session.selected_ship = Some(hold_index);
            assign_selected_command(session, TacticalOrder::HoldPosition, None);
        }

        for ship in &mut session.ships {
            ship.selected = false;
        }
        for fighter in &mut session.fighters {
            fighter.selected = fighter.alive
                && fighter.is_attacker == session.player_is_attacker
                && fighter.fighter_group == 0;
        }
        session.selected_ship = None;
        session.selected_fighter_group = Some(0);
        assign_selected_command(session, TacticalOrder::Recover, None);

        for fighter in &mut session.fighters {
            fighter.selected = false;
        }
        session.ships[ship_index].selected = true;
        session.selected_ship = Some(ship_index);
        session.selected_fighter_group = None;
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Advance one turning capital, one arriving capital, and both final
    /// fighter-recovery transitions through the production executors.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_command_progression_fixture(&mut self) {
        let focus = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let player_ships = session
                .ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| {
                    ship.alive
                        && ship.is_attacker == session.player_is_attacker
                        && ship.subsystem_capacity.engines
                })
                .map(|(index, _)| index)
                .take(2)
                .collect::<Vec<_>>();
            let player_fighters = session
                .fighters
                .iter()
                .enumerate()
                .filter(|(_, fighter)| {
                    fighter.alive && fighter.is_attacker == session.player_is_attacker
                })
                .map(|(index, _)| index)
                .take(2)
                .collect::<Vec<_>>();
            if player_ships.len() < 2 || player_fighters.len() < 2 {
                return;
            }
            let turning = player_ships[0];
            let arriving = player_ships[1];
            let forward_z = if session.ships[turning].identity.is_alliance {
                1.0
            } else {
                -1.0
            };
            for ship in &mut session.ships {
                ship.selected = false;
                ship.source_velocity = TacticalWorldVector::ZERO;
            }
            for fighter in &mut session.fighters {
                fighter.selected = false;
            }

            let turning_position = session.ships[turning].source_position;
            session.ships[turning].selected = true;
            session.ships[turning].maneuverability = 4;
            session.ships[turning].sublight_engine_power = 20.0;
            session.ships[turning].source_forward = TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: forward_z,
            };
            session.ships[turning].source_waypoint = Some(TacticalWorldPosition {
                x: turning_position.x + 50.0,
                y: turning_position.y,
                z: turning_position.z + 50.0 * forward_z,
            });
            session.ships[turning].order = TacticalOrder::LeftHook;

            let arriving_position = session.ships[arriving].source_position;
            session.ships[arriving].sublight_engine_power = 100.0;
            session.ships[arriving].source_forward = TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: forward_z,
            };
            session.ships[arriving].source_desired_forward = session.ships[arriving].source_forward;
            session.ships[arriving].source_waypoint = Some(TacticalWorldPosition {
                x: arriving_position.x,
                y: arriving_position.y,
                z: arriving_position.z + forward_z,
            });
            session.ships[arriving].order = TacticalOrder::Anvil;

            session.advance_original_tactical_movement(ORIGINAL_TACTICAL_STEP_MILLISECONDS);

            let carrier_position = session.ships[arriving].source_position;
            let returning = player_fighters[0];
            session.fighters[returning].fighter_group = 0;
            session.fighters[returning].source_position = TacticalWorldPosition {
                x: carrier_position.x,
                y: carrier_position.y,
                z: carrier_position.z + 2.5 * forward_z,
            };
            session.fighters[returning].sublight_engine_power = 4.0;
            session.fighters[returning].order = TacticalOrder::Recover;
            session.fighters[returning].recovery_state = TacticalFighterRecoveryState::Returning;
            session.fighters[returning].recovery_target = Some(arriving);

            let docking = player_fighters[1];
            session.fighters[docking].fighter_group = 1;
            session.fighters[docking].source_position = carrier_position;
            session.fighters[docking].order = TacticalOrder::Recover;
            session.fighters[docking].recovery_state = TacticalFighterRecoveryState::Docking;
            session.fighters[docking].recovery_target = Some(arriving);
            session.advance_original_fighter_recovery(ORIGINAL_TACTICAL_STEP_MILLISECONDS);

            session.selected_ship = Some(turning);
            session.selected_fighter_group = None;
            session.paused = true;
            self.command_panel = TacticalCommandPanel::Display;
            let object_id = u32::try_from(turning).unwrap_or(u32::MAX).saturating_add(1);
            (object_id, session.ships[turning].source_position.rendered())
        };
        self.asset_renderer.focus_target(focus.0, focus.1);
    }

    /// Exercise both recovered automatic attack-target executors with capital
    /// and fighter owners. Production play reaches these states by committing
    /// the authentic Missions-panel controls.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_attack_targeting_fixture(&mut self) {
        let focus = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let player_ships = session
                .ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| ship.alive && ship.is_attacker == session.player_is_attacker)
                .map(|(index, _)| index)
                .take(2)
                .collect::<Vec<_>>();
            let player_fighters = session
                .fighters
                .iter()
                .enumerate()
                .filter(|(_, fighter)| {
                    fighter.alive && fighter.is_attacker == session.player_is_attacker
                })
                .map(|(index, _)| index)
                .take(2)
                .collect::<Vec<_>>();
            if player_ships.len() < 2 || player_fighters.len() < 2 {
                return;
            }
            for ship in &mut session.ships {
                ship.selected = false;
                ship.attack_target = None;
            }
            for fighter in &mut session.fighters {
                fighter.selected = false;
                fighter.attack_target = None;
            }
            session.fighters[player_fighters[0]].fighter_group = 0;
            session.fighters[player_fighters[1]].fighter_group = 1;

            session.ships[player_ships[0]].selected = true;
            session.selected_ship = Some(player_ships[0]);
            session.selected_fighter_group = None;
            assign_selected_command(session, TacticalOrder::AttackCapitalShips, None);

            session.ships[player_ships[0]].selected = false;
            session.ships[player_ships[1]].selected = true;
            session.selected_ship = Some(player_ships[1]);
            assign_selected_command(session, TacticalOrder::AttackFighters, None);

            session.ships[player_ships[1]].selected = false;
            session.fighters[player_fighters[0]].selected = true;
            session.selected_ship = None;
            session.selected_fighter_group = Some(0);
            assign_selected_command(session, TacticalOrder::AttackCapitalShips, None);

            session.fighters[player_fighters[0]].selected = false;
            session.fighters[player_fighters[1]].selected = true;
            session.selected_fighter_group = Some(1);
            assign_selected_command(session, TacticalOrder::AttackFighters, None);

            session.fighters[player_fighters[1]].selected = false;
            session.ships[player_ships[1]].selected = true;
            session.selected_ship = Some(player_ships[1]);
            session.selected_fighter_group = None;
            session.paused = true;
            self.command_panel = TacticalCommandPanel::Display;

            let selected = player_ships[1];
            let object_id = u32::try_from(selected)
                .unwrap_or(u32::MAX)
                .saturating_add(1);
            (
                object_id,
                session.ships[selected].source_position.rendered(),
            )
        };
        self.asset_renderer.focus_target(focus.0, focus.1);
    }

    /// Invalidate each first-acquired attack target and exercise the recovered
    /// same-class replacement lifecycle. The retired objects remain in the
    /// fixture record so the browser harness can prove why they were skipped.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_attack_target_lifecycle_fixture(&mut self) {
        self.configure_attack_targeting_fixture();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let first_enemy_ship = session
            .ships
            .iter()
            .position(|ship| ship.alive && ship.is_attacker != session.player_is_attacker);
        let first_enemy_fighter = session
            .fighters
            .iter()
            .position(|fighter| fighter.alive && fighter.is_attacker != session.player_is_attacker);
        let (Some(first_enemy_ship), Some(first_enemy_fighter)) =
            (first_enemy_ship, first_enemy_fighter)
        else {
            return;
        };

        session.ships[first_enemy_ship].retreating = true;
        session.fighters[first_enemy_fighter].recovery_state =
            TacticalFighterRecoveryState::Recovered;
        refresh_original_attack_targets(session);
    }

    /// Display one stable frame from every source-selected impact family.
    /// This bridge is compiled only into interface-fixture builds.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_effect_presentation_fixture(&mut self) {
        let focus = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let targets = session
                .ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| ship.is_attacker == session.player_is_attacker)
                .map(|(index, _)| index)
                .take(6)
                .collect::<Vec<_>>();
            let fixtures = [
                (WeaponKind::LaserCannon, OriginalTacticalImpactStage::Hit, 2),
                (WeaponKind::IonCannon, OriginalTacticalImpactStage::Hit, 2),
                (
                    WeaponKind::LaserCannon,
                    OriginalTacticalImpactStage::Damage,
                    3,
                ),
                (
                    WeaponKind::IonCannon,
                    OriginalTacticalImpactStage::Damage,
                    7,
                ),
                (
                    WeaponKind::Turbolaser,
                    OriginalTacticalImpactStage::Damage,
                    2,
                ),
                (
                    WeaponKind::LaserCannon,
                    OriginalTacticalImpactStage::Destroyed,
                    8,
                ),
            ];
            session.impact_effects.clear();
            for (target, (weapon, stage, frame)) in targets.iter().copied().zip(fixtures) {
                queue_original_tactical_impact(&mut session.impact_effects, target, weapon, stage);
                if let Some(effect) = session
                    .impact_effects
                    .iter_mut()
                    .find(|effect| effect.target == target)
                {
                    effect.frame = frame.min(effect.sequence.frame_count().saturating_sub(1));
                }
            }
            session.paused = true;
            targets.get(2).and_then(|&target| {
                session.ships.get(target).map(|ship| {
                    (
                        u32::try_from(target).unwrap_or(u32::MAX).saturating_add(1),
                        ship.source_position.rendered(),
                    )
                })
            })
        };
        if let Some((object_id, position)) = focus {
            self.asset_renderer.focus_target(object_id, position);
        }
    }

    /// Present the remaining source-backed tactical detail states together:
    /// one destroyed hostile hull, the selected command hull's personnel
    /// contents, and a second friendly hull that can receive direct Escort.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_detail_escort_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let player_ships = session
            .ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.alive && ship.is_attacker == session.player_is_attacker)
            .map(|(index, _)| index)
            .take(2)
            .collect::<Vec<_>>();
        let destroyed = session
            .ships
            .iter()
            .enumerate()
            .find(|(_, ship)| ship.alive && ship.is_attacker != session.player_is_attacker)
            .map(|(index, _)| index);
        let ([selected, _escort_target], Some(destroyed)) = (player_ships.as_slice(), destroyed)
        else {
            return;
        };

        for ship in &mut session.ships {
            ship.selected = false;
        }
        for fighter in &mut session.fighters {
            fighter.selected = false;
        }
        session.ships[*selected].selected = true;
        session.selected_ship = Some(*selected);
        session.selected_fighter_group = None;

        // Center the retired hull solely for a fully inspectable source frame
        // in both faction cameras. Production destruction keeps the live
        // object's final position.
        session.ships[destroyed].source_position.x = 0.0;
        session.ships[destroyed].source_position.y = 0.0;
        session.ships[destroyed].source_position.z = 0.0;
        session.ships[destroyed].hull_current = 0;
        session.ships[destroyed].shield = 0;
        session.ships[destroyed].alive = false;
        session.ships[destroyed].selected = false;
        session.impact_effects.clear();
        queue_original_tactical_impact(
            &mut session.impact_effects,
            destroyed,
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Destroyed,
        );
        if let Some(effect) = session
            .impact_effects
            .iter_mut()
            .find(|effect| effect.target == destroyed)
        {
            effect.frame = 8;
        }
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Queue every recovered weapon-event variant so the muted browser journey
    /// can prove the complete TACTICAL.DLL 13033-13054 runtime bank.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_tactical_audio_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.pending_audio_cues.clear();
        for event in [
            TacticalAudioEvent::LaserFire,
            TacticalAudioEvent::TurbolaserFire,
            TacticalAudioEvent::IonFire,
            TacticalAudioEvent::TorpedoFire,
            TacticalAudioEvent::LaserImpact,
            TacticalAudioEvent::IonImpact,
            TacticalAudioEvent::TurbolaserImpact,
            TacticalAudioEvent::TorpedoImpact,
        ] {
            for variant in 0..event.variant_count() {
                session
                    .pending_audio_cues
                    .push(TacticalAudioCue::from_variant(event, variant));
            }
        }
        session.pending_voice_cues.clear();
        for source_event in (0x20..=0x99).chain(0x11e..=0x13c).chain(0x9a..=0x11d) {
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::fixture_source_event(source_event));
        }
    }

    /// Freeze all three projectile shapes plus both shared field families on
    /// live production participants for deterministic browser inspection.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_projectile_field_fixture(&mut self) {
        {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let alliance = session
                .ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| ship.identity.is_alliance)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let empire = session
                .ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| !ship.identity.is_alliance)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if alliance.len() < 3 || empire.len() < 3 {
                return;
            }
            session.weapon_effects.clear();
            let projectile_fixtures = [
                (
                    alliance[0],
                    empire[0],
                    WeaponKind::LaserCannon,
                    ORIGINAL_LASER_PROJECTILE_THRESHOLD,
                ),
                (
                    empire[1],
                    alliance[1],
                    WeaponKind::Turbolaser,
                    ORIGINAL_TURBOLASER_PROJECTILE_THRESHOLD - 1.0,
                ),
                (
                    alliance[2],
                    empire[2],
                    WeaponKind::IonCannon,
                    ORIGINAL_ION_PROJECTILE_THRESHOLD,
                ),
            ];
            for (source, target, weapon, strength) in projectile_fixtures {
                queue_original_tactical_projectile(
                    &mut session.weapon_effects,
                    &session.ships,
                    source,
                    target,
                    weapon,
                    strength,
                );
            }
            for effect in &mut session.weapon_effects {
                effect.elapsed = 0.5;
            }

            session.field_effects.clear();
            session.set_tractor_field(empire[0], alliance[1], true);
            session.set_tractor_field(alliance[0], empire[1], true);
            session.set_gravity_field(alliance[2], empire[1], true);
            session.field_effects[0].frame = 3;
            session.field_effects[1].frame = 5;
            session.paused = true;
        }
        // Widen the source camera field without moving any participant. This
        // keeps both opposing field targets and the projectile paths inside
        // the 444x439 aperture for one deterministic inspection frame.
        for _ in 0..4 {
            self.asset_renderer.zoom_out();
        }
    }

    /// Select one production capital ship and install stable live damage for
    /// the selected-HUD browser gate. This bridge never enters production.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_selected_damage_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(index) = session
            .ships
            .iter()
            .position(|ship| ship.alive && ship.is_attacker == session.player_is_attacker)
        else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
        }
        let ship = &mut session.ships[index];
        ship.selected = true;
        ship.hull_current = (ship.hull_max * 2 / 5).max(1);
        ship.shield = (ship.shield_max / 4).max(0);
        session.selected_ship = Some(index);
        session.selected_fighter_group = None;
        session.paused = true;
    }

    /// Install all five selected-capital condition bands plus source-identified
    /// tractor and gravity locks for deterministic browser inspection.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_subsystem_field_command_fixture(&mut self) {
        self.configure_selected_damage_fixture();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let alliance = session
            .ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.identity.is_alliance)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let empire = session
            .ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| !ship.identity.is_alliance)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if alliance.len() < 3 || empire.len() < 3 {
            return;
        }
        session.field_effects.clear();
        assert!(session.set_tractor_field(empire[0], alliance[1], true));
        assert!(session.set_tractor_field(empire[2], alliance[1], true));
        assert!(session.set_tractor_field(alliance[0], empire[1], true));
        assert!(session.set_gravity_field(alliance[2], empire[1], true));
        // P58F5 is a resource-band presentation fixture. Field changes now
        // refresh production mobility, so install its deliberately chosen
        // five-band display state after those production mutations complete.
        if let Some(index) = session.selected_ship {
            session.ships[index].subsystem_condition = TacticalSubsystemCondition {
                shields: 0,
                weapons: 24,
                tractor: 25,
                engines: 50,
                hyperdrive: 75,
            };
        }
        session.advance_presentational_effects(0.4);
        print_battle_diagnostics(session);
        session.paused = true;
        for _ in 0..4 {
            self.asset_renderer.zoom_out();
        }
    }

    /// Drive one selected production capital through the source combat-damage
    /// path with deterministic rolls for browser inspection. Direct setup is
    /// confined to the test bridge; every recorded hit uses production logic.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_live_subsystem_damage_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(index) = session.ships.iter().position(|ship| {
            ship.alive
                && ship.is_attacker == session.player_is_attacker
                && ship
                    .subsystem_capacity
                    .hit_limits()
                    .iter()
                    .all(|limit| *limit > 0)
        }) else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
        }
        let ship = &mut session.ships[index];
        ship.selected = true;
        ship.hull_current = ship.hull_max;
        ship.shield = ship.shield_max;
        ship.subsystem_damage = TacticalSubsystemDamage::default();
        ship.refresh_subsystem_condition();
        session.selected_ship = Some(index);
        session.selected_fighter_group = None;

        let shield_damage = (session.ships[index].shield_max / 5).max(1);
        apply_original_capital_damage(
            &mut session.ships,
            &mut session.field_effects,
            index,
            shield_damage,
            OriginalTacticalDamageRolls { shield: 0, hull: 0 },
        );
        session.ships[index].shield = 0;
        for (roll, hits) in [(66, 2), (81, 3), (91, 4), (96, 2)] {
            for _ in 0..hits {
                apply_original_capital_damage(
                    &mut session.ships,
                    &mut session.field_effects,
                    index,
                    1,
                    OriginalTacticalDamageRolls {
                        shield: 100,
                        hull: roll,
                    },
                );
            }
        }
        session.paused = true;
    }

    /// Present one source-selected repair while an enemy tractor source lowers
    /// the selected ship's effective sublight condition. Direct setup remains
    /// test-only; repair selection and mobility use production functions.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_subsystem_repair_mobility_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(target) = session.ships.iter().position(|ship| {
            ship.alive
                && ship.is_attacker == session.player_is_attacker
                && ship
                    .subsystem_capacity
                    .hit_limits()
                    .iter()
                    .all(|limit| *limit > 0)
        }) else {
            return;
        };
        let Some(source) = session.ships.iter().enumerate().find_map(|(index, ship)| {
            (index != target && ship.alive && ship.is_attacker != session.player_is_attacker)
                .then_some(index)
        }) else {
            return;
        };

        for ship in &mut session.ships {
            ship.selected = false;
        }
        session.ships[target].selected = true;
        session.ships[target].hull_current = session.ships[target].hull_max;
        session.ships[target].sublight_engine_power = 100.0;
        session.ships[target].maneuver_state_value = Some(9.0);
        session.ships[target].damage_control = 100;
        session.ships[target].subsystem_damage = TacticalSubsystemDamage {
            shields: 1,
            weapons: 2,
            tractor: 1,
            engines: 2,
            hyperdrive: 1,
        };
        session.ships[source].hull_current = session.ships[source].hull_max;
        session.ships[source].subsystem_capacity.tractor = true;
        session.ships[source].tractor_beam_power = 25.0;
        session.ships[source].subsystem_damage.tractor = 0;
        session.field_effects.clear();
        session.subsystem_repairs.clear();
        session.selected_ship = Some(target);
        session.selected_fighter_group = None;
        let _ = session.set_tractor_field(source, target, true);

        if let Some(repair) =
            attempt_original_subsystem_repair(&mut session.ships[target], target, 1, 1)
        {
            session.subsystem_repairs.push(repair);
        }
        refresh_original_subsystem_conditions(&mut session.ships, &session.field_effects);
        let target_object_id = u32::try_from(target).unwrap_or(u32::MAX).saturating_add(1);
        let target_position = session.ships[target].source_position.rendered();
        let (camera_x, camera_y) =
            camera_offset_for_target(session.ships[target].x, session.ships[target].y);
        session.paused = true;
        (self.camera_x, self.camera_y) = (camera_x, camera_y);
        self.asset_renderer
            .focus_target(target_object_id, target_position);
    }

    /// Advance one selected production capital through the recovered
    /// maneuver producer, velocity contract, and 250 ms source integrator.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_maneuver_movement_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(target) = session.ships.iter().position(|ship| {
            ship.alive
                && ship.is_attacker == session.player_is_attacker
                && ship.subsystem_capacity.engines
        }) else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
            ship.source_velocity = TacticalWorldVector::ZERO;
        }
        session.ships[target].selected = true;
        session.ships[target].hull_current = session.ships[target].hull_max;
        session.ships[target].sublight_engine_power = 100.0;
        session.ships[target].subsystem_damage.engines = 1;
        session.ships[target].source_forward = if session.ships[target].identity.is_alliance {
            TacticalWorldVector::SOURCE_FORWARD
        } else {
            TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            }
        };
        session.ships[target].source_desired_forward = session.ships[target].source_forward;
        session.field_effects.clear();
        session.selected_ship = Some(target);
        session.selected_fighter_group = None;
        assert!(session.set_maneuver_state_value(target, Some(4.0)));
        session.advance_original_tactical_movement(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        refresh_original_subsystem_conditions(&mut session.ships, &session.field_effects);

        let target_object_id = u32::try_from(target).unwrap_or(u32::MAX).saturating_add(1);
        let target_position = session.ships[target].source_position.rendered();
        let (camera_x, camera_y) =
            camera_offset_for_target(session.ships[target].x, session.ships[target].y);
        session.paused = true;
        (self.camera_x, self.camera_y) = (camera_x, camera_y);
        self.asset_renderer
            .focus_target(target_object_id, target_position);
    }

    /// End the current battle — clears session. Returns the session for
    /// result processing by the caller.
    pub fn end_battle(&mut self) -> Option<BattleSession> {
        self.dragging_ship = None;
        self.command_panel = TacticalCommandPanel::Display;
        self.session.take()
    }

    /// Consume the one-shot trench-run presentation edge, if the source
    /// tactical result dispatcher resolved one during the latest combat step.
    pub fn take_pending_trench_run_cinematic(&mut self) -> Option<TacticalTrenchRunOutcome> {
        self.session
            .as_mut()
            .and_then(BattleSession::take_pending_trench_run_cinematic)
    }

    /// Drain source-mapped tactical WAVE cues for the application audio
    /// backend. Muted test runs still retain the event/resource diagnostics.
    pub fn take_pending_audio_cues(&mut self) -> Vec<TacticalAudioCue> {
        self.session
            .as_mut()
            .map_or_else(Vec::new, BattleSession::take_pending_audio_cues)
    }

    /// Drain exact VOICEFXA/VOICEFXE tactical acknowledgements for the
    /// application audio backend.
    pub fn take_pending_voice_cues(&mut self) -> Vec<TacticalVoiceCue> {
        self.session
            .as_mut()
            .map_or_else(Vec::new, BattleSession::take_pending_voice_cues)
    }

    /// Project a deterministic strategic auto-resolve outcome onto the live
    /// tactical roster, then enter the same original Battle Results surface
    /// used after a played battle.
    pub fn present_auto_resolve_result(&mut self, result: &SpaceCombatResult) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        if session.attacker_fleet != result.attacker_fleet
            || session.defender_fleet != result.defender_fleet
            || session.system != result.system
        {
            return false;
        }

        for damage in &result.ship_damage {
            let is_attacker = damage.fleet == session.attacker_fleet;
            // Auto-resolve reports an index into its compressed alive-hull
            // snapshot, whereas played tactical results retain original fleet
            // roster slots. The live session was expanded from that same alive
            // sequence, so select its nth hull for presentation only.
            if let Some(ship) = session
                .ships
                .iter_mut()
                .filter(|ship| ship.is_attacker == is_attacker)
                .nth(damage.ship_index)
            {
                ship.hull_current = damage.hull_after.clamp(0, ship.hull_max);
                ship.alive = ship.hull_current > 0;
                if !ship.alive {
                    ship.shield = 0;
                }
                ship.refresh_subsystem_condition();
            }
        }
        for loss in &result.fighter_losses {
            let is_attacker = loss.fleet == session.attacker_fleet;
            for fighter in session.fighters.iter_mut().filter(|fighter| {
                fighter.is_attacker == is_attacker
                    && fighter.fleet_fighter_index == loss.fighter_index
            }) {
                let survives = fighter.fleet_squadron_index < loss.squads_after;
                fighter.alive = survives;
                if !survives {
                    fighter.hull_current = 0.0;
                    fighter.squad_count = 0;
                    fighter.shield = 0.0;
                }
            }
        }
        let winner = match result.winner {
            AutoResolveSide::Attacker => CombatWinner::Attacker,
            AutoResolveSide::Defender => CombatWinner::Defender,
            AutoResolveSide::Draw => CombatWinner::Draw,
        };
        session.queue_battle_outcome_voice(winner);
        session.winner = Some(winner);
        session.phase = BattlePhase::Results;
        session.paused = true;
        session.selected_ship = None;
        session.selected_fighter_group = None;
        self.battle_result_tab = BattleResultTab::Summary;
        self.battle_result_category = BattleResultCategory::CapitalShips;
        self.strategic_results_applied = true;
        true
    }

    /// True when auto-resolve has already mutated the strategic world.
    #[must_use]
    pub const fn strategic_results_applied(&self) -> bool {
        self.strategic_results_applied
    }

    /// Returns true if a battle is currently active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.session.is_some()
    }

    /// Whether the pre-battle source surface still owns tactical entry.
    #[must_use]
    pub const fn battle_alert_open(&self) -> bool {
        self.battle_alert_open
    }

    /// Existing fixture scenarios predate Battle Alert and intentionally
    /// enter the same production session after its Take Command transition.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn dismiss_battle_alert_for_fixture(&mut self) {
        self.battle_alert_open = false;
        self.battle_alert_pressed = None;
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Result of drawing the tactical view — tells main.rs what action to take.
#[derive(Debug, Clone, PartialEq)]
pub enum TacticalAction {
    /// Player is still in the tactical view — no transition needed.
    None,
    /// Player clicked "Begin Battle" — advance from placement to combat phase.
    BeginCombat,
    /// Player clicked "Auto-Resolve" — skip tactical view, auto-resolve.
    AutoResolve,
    /// Battle is over or player clicked "Return to Galaxy" — transition back.
    ReturnToGalaxy,
    /// Leave the Battle Results window and open the battle system.
    OpenBattleSystem(SystemKey),
    /// Leave the Battle Results window and open the surviving victorious fleet.
    OpenBattleFleet(FleetKey),
    /// Toggle combat pause.
    TogglePause,
    /// Set combat speed multiplier.
    SetSpeed(u32),
    /// Retreat selected ships.
    RetreatSelected,
    /// Withdraw every eligible player hull after the Battle Options command.
    WithdrawFromBattle,
    /// The Battle Alert's Retreat (`0x67`, `FUN_00450540`): a simulated
    /// battle in which the player's side may leave (`FUN_00487830` sets the
    /// retreat word 10000.0), not the tactical battle.
    RetreatBeforeBattle,
    /// Route to the original unified Game Options surface.
    OpenGameOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalHudControl {
    TaskForce(u8),
    FighterGroup(u8),
    NavigationSet(u8),
    Pause,
    ZoomIn,
    ZoomOut,
    CameraTarget,
    CameraLeft,
    CameraRight,
    CameraUp,
    CameraDown,
    CameraRecall,
    CameraMemorize,
    HighlightAlliance,
    HighlightEmpire,
    BattleOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalBattleOptionsControl {
    Withdraw,
    SimulateRemainder,
    ObserveToggle,
    GameOptions,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalWithdrawConfirmationControl {
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalCommandControl {
    PreviousCapital,
    NextCapital,
    OpenManeuvers,
    OpenMissions,
    LeftHook,
    RightHook,
    Hammer,
    Anvil,
    HoldPosition,
    ToggleTactic,
    AttackCapitalShips,
    AttackFighters,
    Recover,
    AttackDeathStar,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
struct TacticalHudControlSpec {
    control: TacticalHudControl,
    rect: NativeRect,
    hit_resource: u32,
}

const TACTICAL_HUD_CONTROLS: [TacticalHudControlSpec; 17] = [
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(0),
        rect: NativeRect::new(485.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[0],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(1),
        rect: NativeRect::new(521.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[1],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(2),
        rect: NativeRect::new(560.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[2],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(3),
        rect: NativeRect::new(601.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[3],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::Pause,
        rect: NativeRect::new(560.0, 307.0, 28.0, 21.0),
        hit_resource: resources::tactical::BTN_PAUSE_RUNNING,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::ZoomIn,
        rect: NativeRect::new(486.0, 343.0, 24.0, 24.0),
        hit_resource: resources::tactical::BTN_CAMERA_ZOOM_IN_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::ZoomOut,
        rect: NativeRect::new(603.0, 343.0, 24.0, 24.0),
        hit_resource: resources::tactical::BTN_CAMERA_ZOOM_OUT_NORMAL,
    },
    // The target button is painted above the four overlapping D-pad arms and
    // therefore owns opaque pixels in their shared bounding region.
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraTarget,
        rect: NativeRect::new(538.0, 379.0, 23.0, 23.0),
        hit_resource: resources::tactical::BTN_CAMERA_TARGET_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraLeft,
        rect: NativeRect::new(511.0, 376.0, 42.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_LEFT_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraRight,
        rect: NativeRect::new(557.0, 376.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_RIGHT_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraUp,
        rect: NativeRect::new(537.0, 344.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_UP_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraDown,
        rect: NativeRect::new(537.0, 412.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_DOWN_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraRecall,
        rect: NativeRect::new(484.0, 430.0, 43.0, 25.0),
        hit_resource: resources::tactical::BTN_CAMERA_RECALL_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraMemorize,
        rect: NativeRect::new(585.0, 430.0, 43.0, 25.0),
        hit_resource: resources::tactical::BTN_CAMERA_MEMORIZE_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::HighlightAlliance,
        rect: NativeRect::new(517.0, 304.0, 30.0, 26.0),
        hit_resource: resources::tactical::DIM_ALLIANCE_SHIPS,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::HighlightEmpire,
        rect: NativeRect::new(482.0, 304.0, 30.0, 26.0),
        hit_resource: resources::tactical::DIM_EMPIRE_SHIPS,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::BattleOptions,
        rect: NativeRect::new(606.0, 308.0, 20.0, 20.0),
        hit_resource: resources::tactical::BTN_BATTLE_OPTIONS_NORMAL,
    },
];

#[derive(Debug, Clone, Copy)]
struct TacticalBattleOptionsSpec {
    control: TacticalBattleOptionsControl,
    rect: NativeRect,
    normal_resource: u32,
    pressed_resource: u32,
}

const TACTICAL_BATTLE_OPTIONS_CONTROLS: [TacticalBattleOptionsSpec; 5] = [
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::Withdraw,
        rect: NativeRect::new(497.0, 47.0, 48.0, 34.0),
        normal_resource: resources::tactical::BTN_WITHDRAW_FROM_BATTLE_NORMAL,
        pressed_resource: resources::tactical::BTN_WITHDRAW_FROM_BATTLE_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::SimulateRemainder,
        rect: NativeRect::new(497.0, 114.0, 44.0, 25.0),
        normal_resource: resources::tactical::BTN_SIMULATE_REMAINDER_NORMAL,
        pressed_resource: resources::tactical::BTN_SIMULATE_REMAINDER_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::ObserveToggle,
        rect: NativeRect::new(561.0, 114.0, 45.0, 25.0),
        normal_resource: resources::tactical::BTN_OBSERVE_TOGGLE_NORMAL,
        pressed_resource: resources::tactical::BTN_OBSERVE_TOGGLE_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::GameOptions,
        rect: NativeRect::new(494.0, 203.0, 52.0, 33.0),
        normal_resource: resources::tactical::BTN_GAME_OPTIONS_NORMAL,
        pressed_resource: resources::tactical::BTN_GAME_OPTIONS_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::Close,
        rect: NativeRect::new(565.0, 203.0, 29.0, 33.0),
        normal_resource: resources::tactical::BTN_BATTLE_OPTIONS_CLOSE_NORMAL,
        pressed_resource: resources::tactical::BTN_BATTLE_OPTIONS_CLOSE_PRESSED,
    },
];

#[derive(Debug, Clone, Copy)]
struct TacticalWithdrawConfirmationSpec {
    control: TacticalWithdrawConfirmationControl,
    rect: NativeRect,
    normal_resource: u32,
    pressed_resource: u32,
}

// REBEXE FUN_005f0f40 constructs these controls at panel-local (66, 202)
// and (106, 202), each 27x25, with TACTICAL resources 1113 through 1116.
const TACTICAL_WITHDRAW_CONFIRMATION_CONTROLS: [TacticalWithdrawConfirmationSpec; 2] = [
    TacticalWithdrawConfirmationSpec {
        control: TacticalWithdrawConfirmationControl::Confirm,
        rect: NativeRect::new(547.0, 229.0, 27.0, 25.0),
        normal_resource: resources::tactical::BTN_ASSIGN_CONFIRM_NORMAL,
        pressed_resource: resources::tactical::BTN_ASSIGN_CONFIRM_PRESSED,
    },
    TacticalWithdrawConfirmationSpec {
        control: TacticalWithdrawConfirmationControl::Cancel,
        rect: NativeRect::new(587.0, 229.0, 27.0, 25.0),
        normal_resource: resources::tactical::BTN_ASSIGN_CANCEL_NORMAL,
        pressed_resource: resources::tactical::BTN_ASSIGN_CANCEL_PRESSED,
    },
];

const DEATH_STAR_LASER_GAUGE_RECT: NativeRect = NativeRect::new(474.0, 0.0, 166.0, 25.0);
const DEATH_STAR_LASER_BUTTON_RECT: NativeRect = NativeRect::new(613.0, 2.0, 21.0, 20.0);
const DEATH_STAR_LASER_FILL_RECT: NativeRect = NativeRect::new(508.0, 9.0, 95.0, 6.0);

const TASK_FORCE_BUTTONS: [NativeRect; 8] = [
    NativeRect::new(60.0, 2.0, 25.0, 17.0),
    NativeRect::new(86.0, 2.0, 25.0, 17.0),
    NativeRect::new(112.0, 2.0, 25.0, 17.0),
    NativeRect::new(138.0, 2.0, 25.0, 17.0),
    NativeRect::new(164.0, 2.0, 25.0, 17.0),
    NativeRect::new(190.0, 2.0, 25.0, 17.0),
    NativeRect::new(216.0, 2.0, 25.0, 17.0),
    NativeRect::new(242.0, 2.0, 25.0, 17.0),
];

const FIGHTER_GROUP_BUTTONS: [NativeRect; 4] = [
    NativeRect::new(330.0, 2.0, 25.0, 17.0),
    NativeRect::new(356.0, 2.0, 25.0, 17.0),
    NativeRect::new(382.0, 2.0, 25.0, 17.0),
    NativeRect::new(408.0, 2.0, 25.0, 17.0),
];

const FIGHTER_GROUP_NORMAL_ART: [u32; 4] = [
    resources::tactical::BTN_RED_SQUADRON_NORMAL,
    resources::tactical::BTN_BLUE_SQUADRON_NORMAL,
    resources::tactical::BTN_GREEN_SQUADRON_NORMAL,
    resources::tactical::BTN_GOLD_SQUADRON_NORMAL,
];

const FIGHTER_GROUP_PRESSED_ART: [u32; 4] = [
    resources::tactical::BTN_RED_SQUADRON_PRESSED,
    resources::tactical::BTN_BLUE_SQUADRON_PRESSED,
    resources::tactical::BTN_GREEN_SQUADRON_PRESSED,
    resources::tactical::BTN_GOLD_SQUADRON_PRESSED,
];

const DISPLAY_COMMAND_CONTROLS: [TacticalCommandControl; 4] = [
    TacticalCommandControl::PreviousCapital,
    TacticalCommandControl::NextCapital,
    TacticalCommandControl::OpenMissions,
    TacticalCommandControl::OpenManeuvers,
];

const MANEUVER_COMMAND_CONTROLS: [TacticalCommandControl; 8] = [
    TacticalCommandControl::Hammer,
    TacticalCommandControl::Anvil,
    TacticalCommandControl::LeftHook,
    TacticalCommandControl::RightHook,
    TacticalCommandControl::HoldPosition,
    TacticalCommandControl::ToggleTactic,
    TacticalCommandControl::Confirm,
    TacticalCommandControl::Cancel,
];

const MISSION_COMMAND_CONTROLS: [TacticalCommandControl; 6] = [
    TacticalCommandControl::AttackCapitalShips,
    TacticalCommandControl::AttackFighters,
    TacticalCommandControl::Recover,
    TacticalCommandControl::AttackDeathStar,
    TacticalCommandControl::Confirm,
    TacticalCommandControl::Cancel,
];

fn tactical_command_spec(
    control: TacticalCommandControl,
    player_is_alliance: bool,
) -> (NativeRect, u32) {
    use resources::tactical as art;
    match control {
        TacticalCommandControl::PreviousCapital => (
            NativeRect::new(488.0, 37.0, 11.0, 53.0),
            art::BTN_PREVIOUS_CAPITAL_NORMAL,
        ),
        TacticalCommandControl::NextCapital => (
            NativeRect::new(613.0, 37.0, 11.0, 53.0),
            art::BTN_NEXT_CAPITAL_NORMAL,
        ),
        TacticalCommandControl::OpenMissions => (
            NativeRect::new(493.0, 237.0, 58.0, 22.0),
            art::BTN_MISSIONS_NORMAL,
        ),
        TacticalCommandControl::OpenManeuvers => (
            NativeRect::new(561.0, 237.0, 58.0, 22.0),
            art::BTN_MANEUVERS_TACTICS_NORMAL,
        ),
        TacticalCommandControl::LeftHook => (
            NativeRect::new(494.0, 186.0, 28.0, 28.0),
            art::BTN_LEFT_HOOK_NORMAL,
        ),
        TacticalCommandControl::RightHook => (
            NativeRect::new(524.0, 186.0, 28.0, 28.0),
            art::BTN_RIGHT_HOOK_NORMAL,
        ),
        TacticalCommandControl::Hammer => (
            NativeRect::new(494.0, 156.0, 28.0, 28.0),
            art::BTN_HAMMER_NORMAL,
        ),
        TacticalCommandControl::Anvil => (
            NativeRect::new(524.0, 156.0, 28.0, 28.0),
            art::BTN_ANVIL_NORMAL,
        ),
        TacticalCommandControl::HoldPosition => (
            NativeRect::new(556.0, 171.0, 28.0, 28.0),
            art::BTN_HOLD_POSITION_NORMAL,
        ),
        TacticalCommandControl::ToggleTactic => (
            NativeRect::new(592.0, 159.0, 24.0, 52.0),
            art::BTN_TACTIC_STAND_OFF,
        ),
        TacticalCommandControl::AttackCapitalShips => (
            NativeRect::new(499.0, 156.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_ATTACK_CAPITAL_SHIPS_ALLIANCE_NORMAL
            } else {
                art::BTN_ATTACK_CAPITAL_SHIPS_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::AttackFighters => (
            NativeRect::new(499.0, 186.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_ATTACK_FIGHTERS_ALLIANCE_NORMAL
            } else {
                art::BTN_ATTACK_FIGHTERS_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::Recover => (
            NativeRect::new(567.0, 156.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_RECOVER_ALLIANCE_NORMAL
            } else {
                art::BTN_RECOVER_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::AttackDeathStar => (
            NativeRect::new(567.0, 186.0, 46.0, 26.0),
            art::BTN_ATTACK_DEATH_STAR_NORMAL,
        ),
        TacticalCommandControl::Confirm => (
            NativeRect::new(547.0, 229.0, 27.0, 25.0),
            art::BTN_ASSIGN_CONFIRM_NORMAL,
        ),
        TacticalCommandControl::Cancel => (
            NativeRect::new(587.0, 229.0, 27.0, 25.0),
            art::BTN_ASSIGN_CANCEL_NORMAL,
        ),
    }
}

fn tactical_command_controls(panel: TacticalCommandPanel) -> &'static [TacticalCommandControl] {
    match panel {
        TacticalCommandPanel::Display => &DISPLAY_COMMAND_CONTROLS,
        TacticalCommandPanel::Maneuvers { .. } => &MANEUVER_COMMAND_CONTROLS,
        TacticalCommandPanel::Missions { .. } => &MISSION_COMMAND_CONTROLS,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A containing native bitmap rectangle guarantees finite nonnegative local pixels."
)]
#[expect(
    clippy::too_many_arguments,
    reason = "The source control predicate keeps panel, selection, faction, resource mask, and pointer coordinates explicit."
)]
fn tactical_command_control_at(
    cache: &mut BmpCache,
    panel: TacticalCommandPanel,
    has_selection: bool,
    selected_fighter: bool,
    attack_death_star_enabled: bool,
    player_is_alliance: bool,
    x: f32,
    y: f32,
) -> Option<TacticalCommandControl> {
    if !has_selection {
        return None;
    }
    tactical_command_controls(panel)
        .iter()
        .copied()
        .find(|control| {
            if (matches!(
                *control,
                TacticalCommandControl::PreviousCapital | TacticalCommandControl::NextCapital
            ) && selected_fighter)
                || (*control == TacticalCommandControl::Recover && !selected_fighter)
                || (*control == TacticalCommandControl::AttackDeathStar
                    && !attack_death_star_enabled)
            {
                return false;
            }
            let (rect, resource) = tactical_command_spec(*control, player_is_alliance);
            if !rect.contains(x, y) {
                return false;
            }
            cache.is_resource_hit(
                DllSource::Tactical,
                resource,
                (x - rect.x).floor() as usize,
                (y - rect.y).floor() as usize,
            )
        })
}

#[cfg(test)]
fn tactical_command_rect_control_at(
    panel: TacticalCommandPanel,
    has_selection: bool,
    selected_fighter: bool,
    attack_death_star_enabled: bool,
    x: f32,
    y: f32,
) -> Option<TacticalCommandControl> {
    if !has_selection {
        return None;
    }
    tactical_command_controls(panel)
        .iter()
        .copied()
        .find(|control| {
            (!matches!(
                *control,
                TacticalCommandControl::PreviousCapital | TacticalCommandControl::NextCapital
            ) || !selected_fighter)
                && (*control != TacticalCommandControl::Recover || selected_fighter)
                && (*control != TacticalCommandControl::AttackDeathStar
                    || attack_death_star_enabled)
                && tactical_command_spec(*control, true).0.contains(x, y)
        })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A containing native bitmap rectangle guarantees finite nonnegative local pixels."
)]
fn tactical_hud_control_at(cache: &mut BmpCache, x: f32, y: f32) -> Option<TacticalHudControl> {
    for (index, rect) in TASK_FORCE_BUTTONS.iter().enumerate() {
        if rect.contains(x, y) {
            let pixel_x = (x - rect.x).floor() as usize;
            let pixel_y = (y - rect.y).floor() as usize;
            if cache.is_resource_hit(
                DllSource::Tactical,
                resources::tactical::BTN_TASK_FORCE_NORMAL,
                pixel_x,
                pixel_y,
            ) {
                return Some(TacticalHudControl::TaskForce(
                    u8::try_from(index).unwrap_or(7),
                ));
            }
        }
    }
    for (index, rect) in FIGHTER_GROUP_BUTTONS.iter().enumerate() {
        if rect.contains(x, y) {
            let pixel_x = (x - rect.x).floor() as usize;
            let pixel_y = (y - rect.y).floor() as usize;
            if cache.is_resource_hit(
                DllSource::Tactical,
                FIGHTER_GROUP_NORMAL_ART[index],
                pixel_x,
                pixel_y,
            ) {
                return Some(TacticalHudControl::FighterGroup(
                    u8::try_from(index).unwrap_or(3),
                ));
            }
        }
    }
    TACTICAL_HUD_CONTROLS.iter().find_map(|spec| {
        if !spec.rect.contains(x, y) {
            return None;
        }
        let pixel_x = (x - spec.rect.x).floor() as usize;
        let pixel_y = (y - spec.rect.y).floor() as usize;
        cache
            .is_resource_hit(DllSource::Tactical, spec.hit_resource, pixel_x, pixel_y)
            .then_some(spec.control)
    })
}

fn tactical_battle_options_control_at(
    cache: &mut BmpCache,
    withdraw_enabled: bool,
    x: f32,
    y: f32,
) -> Option<TacticalBattleOptionsControl> {
    TACTICAL_BATTLE_OPTIONS_CONTROLS.iter().find_map(|spec| {
        if spec.control == TacticalBattleOptionsControl::Withdraw && !withdraw_enabled {
            return None;
        }
        if !spec.rect.contains(x, y) {
            return None;
        }
        cache
            .is_resource_hit(
                DllSource::Tactical,
                spec.normal_resource,
                (x - spec.rect.x).floor() as usize,
                (y - spec.rect.y).floor() as usize,
            )
            .then_some(spec.control)
    })
}

fn tactical_withdraw_confirmation_control_at(
    cache: &mut BmpCache,
    x: f32,
    y: f32,
) -> Option<TacticalWithdrawConfirmationControl> {
    TACTICAL_WITHDRAW_CONFIRMATION_CONTROLS
        .iter()
        .find_map(|spec| {
            if !spec.rect.contains(x, y) {
                return None;
            }
            cache
                .is_resource_hit(
                    DllSource::Tactical,
                    spec.normal_resource,
                    (x - spec.rect.x).floor() as usize,
                    (y - spec.rect.y).floor() as usize,
                )
                .then_some(spec.control)
        })
}

/// The mission control's Attack Death Star (`FUN_005ad930`): a selected
/// fighter group and an enemy Death Star the fighters may attack
/// (`FUN_005ba450`), no trench run under way (`FUN_005d0ae0`, the result
/// object's `DAT_006bc978`), fewer Imperial than Alliance fighter groups on
/// the field (the manager's list `+0x988`, class 1, side `+0x168`: 0 is the
/// Alliance, as the voice event `0x84` shows), and none of the player's
/// groups already on the order (`+0x64`/`+0x68` = 6). The manual (p. 151):
/// "there are more Alliance than Imperial fighters currently launched.
/// Only one fighter group at a time may attempt the Death Star mission."
///
/// port: the run is under way from state 5's timer to its result.
fn attack_death_star_enabled(session: &BattleSession) -> bool {
    let (alliance, empire) = session
        .fighters
        .iter()
        .filter(|fighter| fighter.alive)
        .fold((0_usize, 0_usize), |(alliance, empire), fighter| {
            if fighter.identity.is_alliance {
                (alliance + 1, empire)
            } else {
                (alliance, empire + 1)
            }
        });
    session.selected_fighter_group.is_some()
        && session.death_star.is_some_and(|death_star| {
            death_star.attackable() && death_star.is_attacker != session.player_is_attacker
        })
        && session.trench_run_remaining_ms.is_none()
        && empire < alliance
        && !session.fighters.iter().any(|fighter| {
            fighter.alive
                && fighter.is_attacker == session.player_is_attacker
                && fighter.order == TacticalOrder::AttackDeathStar
        })
}

fn player_death_star(session: &BattleSession) -> Option<TacticalDeathStar> {
    session
        .death_star
        .filter(|death_star| death_star.is_attacker == session.player_is_attacker)
}

fn tactical_death_star_laser_ready(session: &BattleSession) -> bool {
    player_death_star(session).is_some_and(|death_star| {
        death_star.operational()
            && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
            && session.death_star_beam.is_none()
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "The containing original bitmap rectangle guarantees finite nonnegative pixels."
)]
fn tactical_death_star_laser_control_at(cache: &mut BmpCache, x: f32, y: f32) -> bool {
    DEATH_STAR_LASER_BUTTON_RECT.contains(x, y)
        && cache.is_resource_hit(
            DllSource::Tactical,
            resources::tactical::DEATH_STAR_LASER_READY,
            (x - DEATH_STAR_LASER_BUTTON_RECT.x).floor() as usize,
            (y - DEATH_STAR_LASER_BUTTON_RECT.y).floor() as usize,
        )
}

fn tactical_withdraw_enabled(session: &BattleSession) -> bool {
    session.ships.iter().any(|ship| {
        ship.alive
            && !ship.retreating
            && ship.is_attacker == session.player_is_attacker
            && ship.subsystem_capacity.hyperdrive > 0
            && ship.subsystem_condition.hyperdrive > 0
    })
}

#[derive(Debug, Clone, Copy)]
struct TacticalGroupHud<'a> {
    task_forces: [bool; 8],
    fighter_groups: [bool; 4],
    selected_task_force: Option<u8>,
    selected_fighter_group: Option<u8>,
    selected_ship_name: Option<&'a str>,
    selected_ship_hud: Option<u32>,
    selected_ship_hull: Option<(i32, i32)>,
    selected_ship_shield: Option<(i32, i32)>,
    selected_ship_subsystems: Option<TacticalSubsystemCondition>,
    selected_ship_contents: &'a [TacticalShipContent],
    selected_fighter_name: Option<&'a str>,
    selected_fighter_hud: Option<u32>,
    selected_order: TacticalOrder,
    selected_tactic: TacticalTactic,
}

impl<'a> TacticalGroupHud<'a> {
    fn from_session(session: &'a BattleSession) -> Self {
        let mut task_forces = [false; 8];
        let mut fighter_groups = [false; 4];
        for ship in &session.ships {
            if ship.alive && ship.is_attacker == session.player_is_attacker {
                task_forces[usize::from(ship.task_force.min(7))] = true;
            }
        }
        for fighter in &session.fighters {
            if fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker == session.player_is_attacker
                && fighter.fighter_group < 4
            {
                fighter_groups[usize::from(fighter.fighter_group)] = true;
            }
        }
        let selected_task_force = selected_player_task_force(session);
        let selected_ship_name = session
            .ships
            .iter()
            .find(|ship| {
                ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
            })
            .map(|ship| ship.name.as_str());
        let selected_ship = session.ships.iter().find(|ship| {
            ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
        });
        let selected_fighter = session.selected_fighter_group.and_then(|group| {
            session.fighters.iter().find(|fighter| {
                fighter.alive
                    && fighter.selected
                    && fighter.is_attacker == session.player_is_attacker
                    && fighter.fighter_group == group
            })
        });
        Self {
            task_forces,
            fighter_groups,
            selected_task_force,
            selected_fighter_group: session.selected_fighter_group,
            selected_ship_name,
            selected_ship_hud: selected_ship
                .and_then(|ship| ship.tactical_resource)
                .map(TacticalCapitalShipResource::hud_resource),
            selected_ship_hull: selected_ship.map(|ship| (ship.hull_current, ship.hull_max)),
            selected_ship_shield: selected_ship.map(|ship| (ship.shield, ship.shield_max)),
            selected_ship_subsystems: selected_ship.map(|ship| ship.subsystem_condition),
            selected_ship_contents: selected_ship.map_or(&[], |ship| ship.contents.as_slice()),
            selected_fighter_name: selected_fighter.map(|fighter| fighter.name.as_str()),
            selected_fighter_hud: selected_fighter
                .and_then(|fighter| fighter.tactical_resource)
                .map(TacticalFighterResource::hud_resource),
            selected_order: selected_fighter.map_or_else(
                || selected_ship.map_or(TacticalOrder::None, |ship| ship.order),
                |fighter| fighter.order,
            ),
            selected_tactic: selected_fighter.map_or_else(
                || selected_ship.map_or(TacticalTactic::StandOff, |ship| ship.tactic),
                |fighter| fighter.tactic,
            ),
        }
    }
}

fn tactical_meter_fraction(current: i32, maximum: i32) -> f32 {
    if maximum <= 0 {
        0.0
    } else {
        (current as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn draw_tactical_selected_meter(
    canvas: TacticalCanvas,
    rect: NativeRect,
    fraction: f32,
    color: Color,
) {
    let (x, y) = canvas.point(rect.x, rect.y);
    draw_rectangle(
        x,
        y,
        rect.width * canvas.scale * fraction.clamp(0.0, 1.0),
        rect.height * canvas.scale,
        color,
    );
}

fn selected_player_task_force(session: &BattleSession) -> Option<u8> {
    if session.selected_fighter_group.is_some() {
        return None;
    }
    let mut selected = session.ships.iter().filter(|ship| {
        ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
    });
    let group = selected.next()?.task_force;
    selected
        .all(|ship| ship.task_force == group)
        .then_some(group)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TacticalFocusAssignment {
    Assigned {
        object_ids: Vec<usize>,
        maximum_targets: usize,
    },
    RejectedMixedTaskForces {
        member_count: usize,
    },
}

/// Apply the original enemy-focus constraint recovered from `FUN_005a24d0`.
/// A capital selection spanning more than one task force is rejected before
/// any target or escort state changes, and the faction-specific source event
/// is queued for the production audio path.
fn assign_selected_capital_focus_target(
    session: &mut BattleSession,
    target_idx: usize,
    append: bool,
) -> TacticalFocusAssignment {
    let selected = session
        .ships
        .iter()
        .enumerate()
        .filter_map(|(index, ship)| {
            (ship.selected && ship.is_attacker == session.player_is_attacker && ship.alive)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    let Some(first) = selected.first().copied() else {
        return TacticalFocusAssignment::Assigned {
            object_ids: Vec::new(),
            maximum_targets: 0,
        };
    };
    let task_force = session.ships[first].task_force;
    if selected
        .iter()
        .any(|index| session.ships[*index].task_force != task_force)
    {
        session
            .pending_voice_cues
            .push(TacticalVoiceCue::target_task_force_rejected(
                session.player_voice_faction(),
            ));
        return TacticalFocusAssignment::RejectedMixedTaskForces {
            member_count: selected.len(),
        };
    }

    let target = TacticalAttackTarget::CapitalShip(target_idx);
    for index in selected.iter().copied() {
        let ship = &mut session.ships[index];
        ship.escort_target = None;
        if append {
            if let Some(position) = ship
                .manual_targets
                .iter()
                .position(|existing| *existing == target)
            {
                ship.manual_targets.remove(position);
            } else {
                ship.manual_targets.push(target);
            }
        } else {
            ship.manual_targets.clear();
            ship.manual_targets.push(target);
        }
        ship.attack_target = ship.manual_targets.first().copied();
    }
    let object_ids = selected
        .iter()
        .filter_map(|index| {
            (session.ships[*index].attack_target == Some(target)).then_some(*index + 1)
        })
        .collect::<Vec<_>>();
    let maximum_targets = selected
        .iter()
        .map(|index| session.ships[*index].manual_targets.len())
        .max()
        .unwrap_or(0);
    TacticalFocusAssignment::Assigned {
        object_ids,
        maximum_targets,
    }
}

fn select_task_force(session: &mut BattleSession, group: u8) -> usize {
    let first = session.ships.iter().position(|ship| {
        ship.alive
            && !ship.retreating
            && ship.is_attacker == session.player_is_attacker
            && ship.task_force == group
    });
    let Some(first) = first else {
        return 0;
    };
    for ship in &mut session.ships {
        ship.selected = ship.alive
            && !ship.retreating
            && ship.is_attacker == session.player_is_attacker
            && ship.task_force == group;
    }
    for fighter in &mut session.fighters {
        fighter.selected = false;
    }
    session.selected_ship = Some(first);
    session.selected_fighter_group = None;
    session.ships.iter().filter(|ship| ship.selected).count()
}

fn assign_selected_to_task_force(session: &mut BattleSession, group: u8) -> usize {
    let mut assigned_groups = [false; 8];
    for ship in &session.ships {
        if ship.alive && ship.is_attacker == session.player_is_attacker {
            assigned_groups[usize::from(ship.task_force.min(7))] = true;
        }
    }
    let next_blank = assigned_groups
        .iter()
        .position(|assigned| !assigned)
        .and_then(|index| u8::try_from(index).ok());
    if !assigned_groups[usize::from(group.min(7))] && next_blank != Some(group.min(7)) {
        return 0;
    }
    let mut assigned = 0;
    for ship in &mut session.ships {
        if ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker {
            ship.task_force = group.min(7);
            assigned += 1;
        }
    }
    if assigned > 0 {
        BattleSession::refresh_original_task_force_geometry(&mut session.ships);
    }
    assigned
}

fn select_fighter_group(session: &mut BattleSession, group: u8) -> bool {
    let assigned = session.fighters.iter().any(|fighter| {
        fighter.alive
            && fighter.squad_count > 0
            && fighter.is_attacker == session.player_is_attacker
            && fighter.fighter_group == group
    });
    if !assigned {
        return false;
    }
    for ship in &mut session.ships {
        ship.selected = false;
    }
    for fighter in &mut session.fighters {
        fighter.selected = fighter.alive
            && fighter.squad_count > 0
            && fighter.is_attacker == session.player_is_attacker
            && fighter.fighter_group == group;
    }
    session.selected_ship = None;
    session.selected_fighter_group = Some(group);
    true
}

fn cycle_selected_capital(session: &mut BattleSession, step: isize) -> Option<(usize, usize, u8)> {
    let current = session.selected_ship.or_else(|| {
        session.ships.iter().position(|ship| {
            ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
        })
    })?;
    let group = session.ships.get(current)?.task_force;
    let eligible = session
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| {
            ship.alive
                && !ship.retreating
                && ship.is_attacker == session.player_is_attacker
                && ship.task_force == group
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let current_position = eligible.iter().position(|index| *index == current)?;
    let count = isize::try_from(eligible.len()).ok()?;
    let next_position = (isize::try_from(current_position).ok()? + step).rem_euclid(count);
    let next = eligible[usize::try_from(next_position).ok()?];
    for (index, ship) in session.ships.iter_mut().enumerate() {
        ship.selected = index == next;
    }
    for fighter in &mut session.fighters {
        fighter.selected = false;
    }
    session.selected_ship = Some(next);
    session.selected_fighter_group = None;
    Some((current, next, group))
}

fn selected_command_values(session: &BattleSession) -> (TacticalOrder, TacticalTactic) {
    if let Some(group) = session.selected_fighter_group {
        if let Some(fighter) = session.fighters.iter().find(|fighter| {
            fighter.alive
                && fighter.selected
                && fighter.is_attacker == session.player_is_attacker
                && fighter.fighter_group == group
        }) {
            return (fighter.order, fighter.tactic);
        }
    }
    session
        .ships
        .iter()
        .find(|ship| ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker)
        .map_or((TacticalOrder::None, TacticalTactic::StandOff), |ship| {
            (ship.order, ship.tactic)
        })
}

fn original_maneuver_waypoint(
    order: TacticalOrder,
    current: TacticalWorldPosition,
    target: TacticalWorldPosition,
) -> Option<TacticalWorldPosition> {
    let delta = source_delta(target, current);
    let (sin, cos) = match order {
        TacticalOrder::LeftHook | TacticalOrder::Anvil => ORIGINAL_MANEUVER_ANGLE.sin_cos(),
        TacticalOrder::RightHook | TacticalOrder::Hammer => (-ORIGINAL_MANEUVER_ANGLE).sin_cos(),
        _ => return None,
    };
    let rotated = match order {
        TacticalOrder::LeftHook | TacticalOrder::RightHook => TacticalWorldVector {
            x: cos.mul_add(delta.x, -sin * delta.z),
            y: delta.y,
            z: cos.mul_add(delta.z, sin * delta.x),
        },
        TacticalOrder::Hammer | TacticalOrder::Anvil => TacticalWorldVector {
            x: delta.x,
            y: cos.mul_add(delta.y, sin * delta.z),
            z: cos.mul_add(delta.z, -sin * delta.y),
        },
        _ => return None,
    };
    Some(TacticalWorldPosition {
        x: current.x + rotated.x * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
        y: current.y + rotated.y * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
        z: current.z + rotated.z * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
    })
}

fn original_maneuver_target(
    ships: &[TacticalShip],
    source: usize,
    order: TacticalOrder,
) -> Option<TacticalWorldPosition> {
    let source_ship = ships.get(source)?;
    let opponents = ships
        .iter()
        .filter(|ship| ship.alive && ship.is_attacker != source_ship.is_attacker)
        .collect::<Vec<_>>();
    let count = opponents.len() as f32;
    if count == 0.0 {
        return None;
    }
    let anchor = opponents
        .iter()
        .fold(TacticalWorldPosition::ORIGIN, |sum, ship| {
            TacticalWorldPosition {
                x: sum.x + ship.source_position.x / count,
                y: sum.y + ship.source_position.y / count,
                z: sum.z + ship.source_position.z / count,
            }
        });
    let delta = source_delta(anchor, source_ship.source_position);
    let selector = match order {
        TacticalOrder::LeftHook => TacticalWorldVector {
            x: -delta.z,
            y: delta.y,
            z: delta.x,
        },
        TacticalOrder::RightHook => TacticalWorldVector {
            x: delta.z,
            y: delta.y,
            z: -delta.x,
        },
        TacticalOrder::Hammer => TacticalWorldVector {
            x: delta.x,
            y: -delta.y,
            z: delta.z,
        },
        TacticalOrder::Anvil => delta,
        _ => return None,
    }
    .normalized();
    let mut selected = None;
    let mut selected_score = f32::NEG_INFINITY;
    for opponent in opponents {
        let position = opponent.source_position;
        let score = selector.dot(TacticalWorldVector {
            x: position.x,
            y: position.y,
            z: position.z,
        });
        if score > selected_score {
            selected = Some(position);
            selected_score = score;
        }
    }
    selected
}

fn begin_original_capital_order(
    session: &mut BattleSession,
    selected: &[usize],
    order: TacticalOrder,
) {
    for &index in selected {
        if order == TacticalOrder::HoldPosition {
            let ship = &mut session.ships[index];
            ship.source_waypoint = None;
            ship.navigation_route.clear();
            ship.source_desired_forward = TacticalWorldVector::ZERO;
            ship.source_velocity = TacticalWorldVector::ZERO;
            continue;
        }
        if !matches!(
            order,
            TacticalOrder::LeftHook
                | TacticalOrder::RightHook
                | TacticalOrder::Hammer
                | TacticalOrder::Anvil
        ) {
            continue;
        }
        let current = session.ships[index].source_position;
        let waypoint = original_maneuver_target(&session.ships, index, order)
            .and_then(|target| original_maneuver_waypoint(order, current, target));
        let ship = &mut session.ships[index];
        ship.navigation_route.clear();
        if let Some(waypoint) = waypoint {
            ship.source_waypoint = Some(waypoint);
            ship.source_desired_forward = source_delta(waypoint, current).normalized();
        } else {
            // `FUN_005ca6d0` returns the object to no-orders when no executor
            // target can be created.
            ship.order = TacticalOrder::None;
            ship.source_waypoint = None;
            ship.source_desired_forward = TacticalWorldVector::ZERO;
            ship.source_velocity = TacticalWorldVector::ZERO;
        }
    }
}

/// Apply the original direct Escort order (source order code 1). The manual
/// assigns it by right-clicking a friendly capital ship, outside either
/// assignment panel. The recovered order-1 executor retains that object in its
/// target slot.
fn assign_selected_escort(session: &mut BattleSession, target: usize) -> (usize, usize) {
    let Some(target_ship) = session
        .ships
        .get(target)
        .filter(|ship| ship.alive && !ship.retreating)
    else {
        return (0, 0);
    };
    let target_side = target_ship.is_attacker;
    if target_side != session.player_is_attacker {
        return (0, 0);
    }

    let mut capitals = 0;
    for (index, ship) in session.ships.iter_mut().enumerate() {
        if index != target
            && ship.alive
            && !ship.retreating
            && ship.selected
            && ship.is_attacker == target_side
        {
            ship.order = TacticalOrder::Escort;
            ship.escort_target = Some(target);
            ship.attack_target = None;
            ship.manual_targets.clear();
            ship.navigation_route.clear();
            capitals += 1;
        }
    }

    let mut fighters = 0;
    for fighter in &mut session.fighters {
        if fighter.alive
            && fighter.squad_count > 0
            && fighter.selected
            && fighter.is_attacker == target_side
        {
            fighter.order = TacticalOrder::Escort;
            fighter.escort_target = Some(target);
            fighter.attack_target = None;
            fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
            fighter.recovery_target = None;
            fighters += 1;
        }
    }
    (capitals, fighters)
}

/// Run the target-acquisition half of `FUN_005d0b10` and `FUN_005d0bb0`.
/// Both source executors traverse one stable tactical-object list, filter it
/// by the vtable type code (capital 0, fighter 1), accept the first eligible
/// hostile object, mark the executor active, and dispatch event `0x36`.
fn begin_original_attack_order(
    session: &mut BattleSession,
    selected_capitals: &[usize],
    selected_fighters: &[usize],
    order: TacticalOrder,
) {
    if order == TacticalOrder::AttackDeathStar {
        let Some(death_star) = session
            .death_star
            .filter(|death_star| death_star.hull > 0.0 && !death_star.destroyed)
        else {
            return;
        };
        let Some((fighter_group, fighter_is_attacker)) =
            selected_fighters.iter().find_map(|&index| {
                session
                    .fighters
                    .get(index)
                    .filter(|fighter| {
                        fighter.alive
                            && fighter.squad_count > 0
                            && fighter.is_attacker != death_star.is_attacker
                    })
                    .map(|fighter| (fighter.fighter_group, fighter.is_attacker))
            })
        else {
            return;
        };
        if session.trench_run_remaining_ms.is_some() || session.trench_run_outcome.is_some() {
            return;
        }
        session.trench_run_remaining_ms = Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
        session.trench_run_group = Some(fighter_group);
        if session.player_voice_faction() == TacticalVoiceFaction::Alliance
            && fighter_is_attacker == session.player_is_attacker
        {
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::trench_run_started(fighter_group));
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::trench_run_committed(fighter_group));
        } else if session.player_voice_faction() == TacticalVoiceFaction::Empire
            && death_star.is_attacker == session.player_is_attacker
        {
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::death_star_under_attack());
        } else {
            session.trench_run_remaining_ms = None;
            session.trench_run_group = None;
            return;
        }
        session.prepare_original_trench_run();
        return;
    }

    if !matches!(
        order,
        TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
    ) {
        return;
    }

    for &index in selected_capitals {
        let source_is_attacker = session.ships[index].is_attacker;
        let current = session.ships[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        let target = first_original_attack_target(session, source_is_attacker, order);
        session.ships[index].attack_target = target;
    }
    for &index in selected_fighters {
        let source_is_attacker = session.fighters[index].is_attacker;
        let current = session.fighters[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        let target = first_original_attack_target(session, source_is_attacker, order);
        session.fighters[index].attack_target = target;
    }
}

fn begin_original_fighter_recovery(session: &mut BattleSession, selected: &[usize]) {
    let player_is_attacker = session.player_is_attacker;
    let player_faction = session.player_voice_faction();
    let mut warned_groups = [false; 4];
    let mut capacity_cues = Vec::new();
    let mut carrier_slots = session
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.alive && ship.fighter_capacity > 0)
        .map(|(index, ship)| (index, ship.is_attacker, ship.fighter_capacity))
        .collect::<Vec<_>>();
    for (index, fighter) in session.fighters.iter().enumerate() {
        if selected.contains(&index)
            || !matches!(
                fighter.recovery_state,
                TacticalFighterRecoveryState::Reserved
                    | TacticalFighterRecoveryState::Returning
                    | TacticalFighterRecoveryState::Docking
            )
        {
            continue;
        }
        if let Some((_, _, remaining)) = fighter.recovery_target.and_then(|target| {
            carrier_slots
                .iter_mut()
                .find(|(carrier, _, _)| *carrier == target)
        }) {
            *remaining = remaining.saturating_sub(1);
        }
    }
    for &fighter_index in selected {
        let fighter = &mut session.fighters[fighter_index];
        // `FUN_005cf940` always resets the state before `FUN_005cf980`
        // reserves a compatible carrier and begins the inbound leg.
        fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
        fighter.recovery_target = None;
        if let Some((carrier, _, remaining)) = carrier_slots
            .iter_mut()
            .find(|(_, side, remaining)| *side == fighter.is_attacker && *remaining > 0)
        {
            fighter.recovery_state = TacticalFighterRecoveryState::Reserved;
            fighter.recovery_target = Some(*carrier);
            *remaining -= 1;
            fighter.recovery_state = TacticalFighterRecoveryState::Returning;
        } else {
            let group = usize::from(fighter.fighter_group);
            if fighter.is_attacker == player_is_attacker
                && group < warned_groups.len()
                && !warned_groups[group]
            {
                warned_groups[group] = true;
                capacity_cues.push(TacticalVoiceCue::fighter_recovery_capacity(
                    player_faction,
                    fighter.fighter_group,
                ));
            }
        }
    }
    session.pending_voice_cues.extend(capacity_cues);
}

fn assign_selected_command(
    session: &mut BattleSession,
    order: TacticalOrder,
    tactic: Option<TacticalTactic>,
) -> (usize, usize) {
    let mut capital_members = 0;
    let mut fighter_members = 0;
    let mut selected_capitals = Vec::new();
    let mut selected_fighters = Vec::new();
    if session.selected_fighter_group.is_some() {
        for (index, fighter) in session.fighters.iter_mut().enumerate() {
            if fighter.alive
                && fighter.selected
                && fighter.is_attacker == session.player_is_attacker
            {
                fighter.order = order;
                fighter.escort_target = None;
                if let Some(tactic) = tactic {
                    fighter.tactic = tactic;
                }
                if order != TacticalOrder::Recover {
                    fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
                    fighter.recovery_target = None;
                }
                selected_fighters.push(index);
                fighter_members += 1;
            }
        }
    } else {
        for (index, ship) in session.ships.iter_mut().enumerate() {
            if ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker {
                ship.order = order;
                ship.escort_target = None;
                if let Some(tactic) = tactic {
                    ship.tactic = tactic;
                }
                selected_capitals.push(index);
                capital_members += 1;
            }
        }
    }
    begin_original_capital_order(session, &selected_capitals, order);
    begin_original_attack_order(session, &selected_capitals, &selected_fighters, order);
    if order == TacticalOrder::Recover {
        begin_original_fighter_recovery(session, &selected_fighters);
    }
    (capital_members, fighter_members)
}

fn queue_selected_command_voice(
    session: &mut BattleSession,
    panel: TacticalCommandPanel,
    order: TacticalOrder,
    formation_changed: bool,
) {
    let faction = if player_is_alliance(session) {
        TacticalVoiceFaction::Alliance
    } else {
        TacticalVoiceFaction::Empire
    };
    let cue = if let Some(group) = session.selected_fighter_group {
        match (panel, order) {
            // `FUN_005d0340` dispatches Attack Death Star entry from the
            // order executor because the local listener may be either the
            // Alliance attackers or the Imperial station owner. The shared
            // executor above has already queued that perspective-correct cue.
            (TacticalCommandPanel::Missions { .. }, TacticalOrder::AttackDeathStar) => return,
            (TacticalCommandPanel::Maneuvers { .. }, _) => {
                TacticalVoiceCue::fighter_maneuver(faction, group)
            }
            (TacticalCommandPanel::Missions { .. }, _)
                if matches!(
                    order,
                    TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
                ) =>
            {
                TacticalVoiceCue::fighter_attack(faction, group)
            }
            (TacticalCommandPanel::Missions { .. }, _) => {
                TacticalVoiceCue::fighter_mission(faction, group)
            }
            (TacticalCommandPanel::Display, _) => return,
        }
    } else {
        let Some(group) = selected_player_task_force(session) else {
            return;
        };
        match panel {
            TacticalCommandPanel::Maneuvers { .. } if formation_changed => {
                TacticalVoiceCue::capital_formation(faction, group)
            }
            TacticalCommandPanel::Maneuvers { .. } => {
                TacticalVoiceCue::capital_maneuver(faction, group)
            }
            TacticalCommandPanel::Missions { .. }
                if matches!(
                    order,
                    TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
                ) =>
            {
                TacticalVoiceCue::capital_attack(faction, group)
            }
            TacticalCommandPanel::Missions { .. } => {
                TacticalVoiceCue::capital_mission(faction, group)
            }
            TacticalCommandPanel::Display => return,
        }
    };
    session.pending_voice_cues.push(cue);
}

fn pressed_tactical_hud_control(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
) -> Option<TacticalHudControl> {
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    tactical_hud_control_at(cache, x, y)
}

fn pressed_tactical_battle_options_control(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    withdraw_enabled: bool,
) -> Option<TacticalBattleOptionsControl> {
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    tactical_battle_options_control_at(cache, withdraw_enabled, x, y)
}

fn pressed_tactical_withdraw_confirmation_control(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
) -> Option<TacticalWithdrawConfirmationControl> {
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    tactical_withdraw_confirmation_control_at(cache, x, y)
}

fn pressed_tactical_command_control(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    panel: TacticalCommandPanel,
    has_selection: bool,
    selected_fighter: bool,
    attack_death_star_enabled: bool,
    player_is_alliance: bool,
) -> Option<TacticalCommandControl> {
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    tactical_command_control_at(
        cache,
        panel,
        has_selection,
        selected_fighter,
        attack_death_star_enabled,
        player_is_alliance,
        x,
        y,
    )
}

#[cfg(test)]
fn tactical_hud_rect_control_at(x: f32, y: f32) -> Option<TacticalHudControl> {
    TACTICAL_HUD_CONTROLS
        .iter()
        .find_map(|spec| spec.rect.contains(x, y).then_some(spec.control))
}

fn draw_tactical_bitmap(cache: &mut BmpCache, id: u32, canvas: TacticalCanvas, x: f32, y: f32) {
    draw_original_bitmap(cache, DllSource::Tactical, id, canvas, x, y, None);
}

fn draw_strategy_bitmap(cache: &mut BmpCache, id: u32, canvas: TacticalCanvas, x: f32, y: f32) {
    draw_original_bitmap(cache, DllSource::Strategy, id, canvas, x, y, None);
}

fn draw_original_bitmap(
    cache: &mut BmpCache,
    source: DllSource,
    id: u32,
    canvas: TacticalCanvas,
    x: f32,
    y: f32,
    source_rect: Option<Rect>,
) {
    let Some(texture) = cache.get_macroquad_original(source, id) else {
        return;
    };
    let (screen_x, screen_y) = canvas.point(x, y);
    let source_width = source_rect.map_or_else(|| texture.width(), |rect| rect.w);
    let source_height = source_rect.map_or_else(|| texture.height(), |rect| rect.h);
    draw_texture_ex(
        texture,
        screen_x,
        screen_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(
                source_width * canvas.scale,
                source_height * canvas.scale,
            )),
            source: source_rect,
            ..Default::default()
        },
    );
}

/// Cache every glyph and size that the macroquad tactical pass can emit before
/// any texture-backed draw command is queued. Macroquad 0.4.14 replaces its
/// shared font-atlas texture when an uncached glyph expands the atlas, which
/// invalidates earlier commands in the same WebGL frame.
fn prewarm_tactical_font_atlas(
    session: &BattleSession,
    canvas: TacticalCanvas,
    warmed_sizes: &mut HashSet<u16>,
) {
    // The projected unit labels clamp to this complete integer range as the
    // camera zoom changes. Prewarm every value up front: adding only the
    // current zoom's sizes can repack Macroquad's shared glyph atlas during a
    // later zoom and perturb otherwise-static HUD text at fractional scales.
    let mut sizes = vec![7_u16, 8, 9, 10, 11, 12, 13, 14];
    sizes.extend([
        (10.0 * canvas.scale) as u16,
        (11.0 * canvas.scale) as u16,
        (12.0 * canvas.scale) as u16,
        (13.0 * canvas.scale) as u16,
        (14.0 * canvas.scale) as u16,
        (16.0 * canvas.scale) as u16,
        (18.0 * canvas.scale) as u16,
    ]);
    sizes.sort_unstable();
    if sizes.iter().all(|size| warmed_sizes.contains(size)) {
        return;
    }

    let mut characters: std::collections::BTreeSet<_> =
        Font::latin_character_list().into_iter().collect();
    for text in [
        "0123456789x#",
        "Task Force No Orders RETREAT Battle Paused.",
        "Red Group Blue Group Green Group Gold Group",
        "Battle at Alliance Imperial Forces Capital Ships Fighters Troops Personnel",
        "Operational Damaged Destroyed Go Directly To System Fleet victorious withdrawn",
        "Withdraw Confirmation Are you sure you wish to order a fleet withdraw?",
    ] {
        characters.extend(text.chars());
    }
    characters.extend(session.system_name.chars());
    for ship in &session.ships {
        characters.extend(ship.name.chars());
    }
    for fighter in &session.fighters {
        characters.extend(fighter.name.chars());
    }
    let sample: String = characters.into_iter().collect();

    let mut previous = None;
    for size in sizes {
        if size > 0 && previous != Some(size) && warmed_sizes.insert(size) {
            measure_text(&sample, None, size, 1.0);
            previous = Some(size);
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "The 640x480 tactical source canvas uses integer device-pixel clipping."
)]
fn set_tactical_aperture_clip(aperture: Option<NativeRect>) {
    let clip = aperture.map(|rect| {
        (
            rect.x.round() as i32,
            rect.y.round() as i32,
            rect.width.round() as i32,
            rect.height.round() as i32,
        )
    });
    // SAFETY: macroquad exposes its immediate drawing state through this API.
    // The clip is always reset before original HUD controls are painted.
    unsafe {
        get_internal_gl().quad_gl.scissor(clip);
    }
}

fn draw_tactical_assignment_button(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    control: TacticalCommandControl,
    resource: u32,
) {
    let rect = tactical_command_spec(control, true).0;
    draw_tactical_bitmap(cache, resource, canvas, rect.x, rect.y);
}

fn draw_tactical_assignment_panel(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    player_is_alliance: bool,
    panel: TacticalCommandPanel,
    selected_fighter: bool,
    attack_death_star_enabled: bool,
    pressed: Option<TacticalCommandControl>,
) {
    use resources::tactical as art;
    match panel {
        TacticalCommandPanel::Display => {}
        TacticalCommandPanel::Maneuvers {
            pending_order,
            pending_tactic,
        } => {
            draw_tactical_bitmap(cache, art::MANEUVERS_PANEL, canvas, 481.0, 27.0);
            if let Some(graphic) = pending_order.maneuver_graphic() {
                draw_tactical_bitmap(cache, graphic, canvas, 495.0, 41.0);
            }
            for (control, normal, selected) in [
                (
                    TacticalCommandControl::LeftHook,
                    art::BTN_LEFT_HOOK_NORMAL,
                    pending_order == TacticalOrder::LeftHook,
                ),
                (
                    TacticalCommandControl::RightHook,
                    art::BTN_RIGHT_HOOK_NORMAL,
                    pending_order == TacticalOrder::RightHook,
                ),
                (
                    TacticalCommandControl::Hammer,
                    art::BTN_HAMMER_NORMAL,
                    pending_order == TacticalOrder::Hammer,
                ),
                (
                    TacticalCommandControl::Anvil,
                    art::BTN_ANVIL_NORMAL,
                    pending_order == TacticalOrder::Anvil,
                ),
                (
                    TacticalCommandControl::HoldPosition,
                    art::BTN_HOLD_POSITION_NORMAL,
                    pending_order == TacticalOrder::HoldPosition,
                ),
            ] {
                let active = selected || pressed == Some(control);
                draw_tactical_assignment_button(
                    cache,
                    canvas,
                    control,
                    if active { normal + 1 } else { normal },
                );
            }
            draw_tactical_assignment_button(
                cache,
                canvas,
                TacticalCommandControl::ToggleTactic,
                if pending_tactic == TacticalTactic::Surround {
                    art::BTN_TACTIC_SURROUND
                } else {
                    art::BTN_TACTIC_STAND_OFF
                },
            );
            for (control, normal, pressed_art) in [
                (
                    TacticalCommandControl::Confirm,
                    art::BTN_ASSIGN_CONFIRM_NORMAL,
                    art::BTN_ASSIGN_CONFIRM_PRESSED,
                ),
                (
                    TacticalCommandControl::Cancel,
                    art::BTN_ASSIGN_CANCEL_NORMAL,
                    art::BTN_ASSIGN_CANCEL_PRESSED,
                ),
            ] {
                draw_tactical_assignment_button(
                    cache,
                    canvas,
                    control,
                    if pressed == Some(control) {
                        pressed_art
                    } else {
                        normal
                    },
                );
            }
        }
        TacticalCommandPanel::Missions { pending_order } => {
            draw_tactical_bitmap(cache, art::MISSIONS_PANEL, canvas, 481.0, 27.0);
            draw_tactical_bitmap(
                cache,
                pending_order
                    .mission_graphic(player_is_alliance)
                    .unwrap_or(art::MISSIONS_HUD_EMPTY),
                canvas,
                495.0,
                41.0,
            );
            for (control, normal, pressed_art, selected) in [
                (
                    TacticalCommandControl::AttackCapitalShips,
                    if player_is_alliance {
                        art::BTN_ATTACK_CAPITAL_SHIPS_ALLIANCE_NORMAL
                    } else {
                        art::BTN_ATTACK_CAPITAL_SHIPS_EMPIRE_NORMAL
                    },
                    if player_is_alliance {
                        art::BTN_ATTACK_CAPITAL_SHIPS_ALLIANCE_PRESSED
                    } else {
                        art::BTN_ATTACK_CAPITAL_SHIPS_EMPIRE_PRESSED
                    },
                    pending_order == TacticalOrder::AttackCapitalShips,
                ),
                (
                    TacticalCommandControl::AttackFighters,
                    if player_is_alliance {
                        art::BTN_ATTACK_FIGHTERS_ALLIANCE_NORMAL
                    } else {
                        art::BTN_ATTACK_FIGHTERS_EMPIRE_NORMAL
                    },
                    if player_is_alliance {
                        art::BTN_ATTACK_FIGHTERS_ALLIANCE_PRESSED
                    } else {
                        art::BTN_ATTACK_FIGHTERS_EMPIRE_PRESSED
                    },
                    pending_order == TacticalOrder::AttackFighters,
                ),
                (
                    TacticalCommandControl::Recover,
                    if player_is_alliance {
                        art::BTN_RECOVER_ALLIANCE_NORMAL
                    } else {
                        art::BTN_RECOVER_EMPIRE_NORMAL
                    },
                    if player_is_alliance {
                        art::BTN_RECOVER_ALLIANCE_PRESSED
                    } else {
                        art::BTN_RECOVER_EMPIRE_PRESSED
                    },
                    pending_order == TacticalOrder::Recover,
                ),
            ] {
                let resource = if control == TacticalCommandControl::Recover && !selected_fighter {
                    if player_is_alliance {
                        art::BTN_RECOVER_ALLIANCE_DISABLED
                    } else {
                        art::BTN_RECOVER_EMPIRE_DISABLED
                    }
                } else if selected || pressed == Some(control) {
                    pressed_art
                } else {
                    normal
                };
                draw_tactical_assignment_button(cache, canvas, control, resource);
            }
            draw_tactical_assignment_button(
                cache,
                canvas,
                TacticalCommandControl::AttackDeathStar,
                if !attack_death_star_enabled {
                    art::BTN_ATTACK_DEATH_STAR_DISABLED
                } else if pending_order == TacticalOrder::AttackDeathStar
                    || pressed == Some(TacticalCommandControl::AttackDeathStar)
                {
                    art::BTN_ATTACK_DEATH_STAR_PRESSED
                } else {
                    art::BTN_ATTACK_DEATH_STAR_NORMAL
                },
            );
            for (control, normal, pressed_art) in [
                (
                    TacticalCommandControl::Confirm,
                    art::BTN_ASSIGN_CONFIRM_NORMAL,
                    art::BTN_ASSIGN_CONFIRM_PRESSED,
                ),
                (
                    TacticalCommandControl::Cancel,
                    art::BTN_ASSIGN_CANCEL_NORMAL,
                    art::BTN_ASSIGN_CANCEL_PRESSED,
                ),
            ] {
                draw_tactical_assignment_button(
                    cache,
                    canvas,
                    control,
                    if pressed == Some(control) {
                        pressed_art
                    } else {
                        normal
                    },
                );
            }
        }
    }
}

fn draw_tactical_withdraw_confirmation(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    pressed: Option<TacticalWithdrawConfirmationControl>,
) {
    use resources::tactical as art;

    draw_tactical_bitmap(cache, art::WITHDRAW_CONFIRMATION_PANEL, canvas, 481.0, 27.0);
    for spec in TACTICAL_WITHDRAW_CONFIRMATION_CONTROLS {
        draw_tactical_bitmap(
            cache,
            if pressed == Some(spec.control) {
                spec.pressed_resource
            } else {
                spec.normal_resource
            },
            canvas,
            spec.rect.x,
            spec.rect.y,
        );
    }

    // FUN_005f1050 places TEXTTACT 56772 at (20, 28) and TEXTTACT 56771
    // at (14, 92) in the 149x236 panel. Macroquad takes a text baseline, so
    // add the recovered ten-pixel font height while preserving those origins.
    let font_size = 10.0 * canvas.scale;
    let text_color = Color::new(0.84, 0.92, 0.84, 1.0);
    for (text, x, top) in [
        ("Withdraw Confirmation", 501.0, 55.0),
        ("Are you sure you wish to", 495.0, 119.0),
        ("order a fleet withdraw?", 495.0, 131.0),
    ] {
        let (screen_x, screen_y) = canvas.point(x, top + 10.0);
        draw_text(text, screen_x, screen_y, font_size, text_color);
    }
}

/// Paint only resource-backed battle chrome. Unmapped command groups remain
/// visible but are not passed off as implemented interactions.
#[expect(
    clippy::too_many_arguments,
    reason = "The original HUD paints one fixed canvas from explicit battle state."
)]
#[expect(
    clippy::too_many_lines,
    reason = "The authored tactical chrome is one fixed 640×480 source composition."
)]
fn draw_original_tactical_hud(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    player_is_alliance: bool,
    groups: TacticalGroupHud<'_>,
    attack_death_star_enabled: bool,
    paused: bool,
    highlight_alliance: bool,
    highlight_empire: bool,
    navigation_sets_visible: [bool; 4],
    pressed_control: Option<TacticalHudControl>,
    command_panel: TacticalCommandPanel,
    pressed_command: Option<TacticalCommandControl>,
    battle_options_open: bool,
    withdraw_confirmation_open: bool,
    player_observing: bool,
    withdraw_enabled: bool,
    pressed_options: Option<TacticalBattleOptionsControl>,
    pressed_withdraw_confirmation: Option<TacticalWithdrawConfirmationControl>,
    death_star: Option<TacticalDeathStar>,
    death_star_laser_pressed: bool,
    death_star_targeting: bool,
) {
    use resources::tactical as art;

    if let Some(death_star) = death_star {
        draw_tactical_bitmap(
            cache,
            art::DEATH_STAR_LASER_GAUGE,
            canvas,
            DEATH_STAR_LASER_GAUGE_RECT.x,
            DEATH_STAR_LASER_GAUGE_RECT.y,
        );
        let fraction =
            (death_star.laser_charge / ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE).clamp(0.0, 1.0);
        if fraction > 0.0 {
            let (x, y) = canvas.point(DEATH_STAR_LASER_FILL_RECT.x, DEATH_STAR_LASER_FILL_RECT.y);
            draw_rectangle(
                x,
                y,
                DEATH_STAR_LASER_FILL_RECT.width * canvas.scale * fraction,
                DEATH_STAR_LASER_FILL_RECT.height * canvas.scale,
                Color::new(0.0, 0.82, 0.0, 1.0),
            );
        }
        let ready = death_star.operational()
            && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE;
        let active =
            death_star_laser_pressed || death_star_targeting || death_star.action_committed;
        draw_tactical_bitmap(
            cache,
            if active {
                art::DEATH_STAR_LASER_FIRED
            } else if ready {
                art::DEATH_STAR_LASER_READY
            } else {
                art::DEATH_STAR_LASER_LOADING
            },
            canvas,
            DEATH_STAR_LASER_BUTTON_RECT.x,
            DEATH_STAR_LASER_BUTTON_RECT.y,
        );
    }

    // The top task-force and fighter-group strips occupy slots in bitmap 1000.
    draw_tactical_bitmap(
        cache,
        if player_is_alliance {
            art::TASK_FORCES_ALLIANCE
        } else {
            art::TASK_FORCES_EMPIRE
        },
        canvas,
        10.0,
        1.0,
    );
    for index in 0..8_u8 {
        let assigned = groups.task_forces[usize::from(index)];
        let active = assigned
            && (groups.selected_task_force == Some(index)
                || pressed_control == Some(TacticalHudControl::TaskForce(index)));
        draw_tactical_bitmap(
            cache,
            if active {
                art::BTN_TASK_FORCE_PRESSED
            } else if assigned {
                art::BTN_TASK_FORCE_NORMAL
            } else {
                art::BTN_TASK_FORCE_UNASSIGNED
            },
            canvas,
            60.0 + f32::from(index) * 26.0,
            2.0,
        );
        if assigned {
            let label = (index + 1).to_string();
            let font_size = 13.0 * canvas.scale;
            let width = measure_text(&label, None, font_size as u16, 1.0).width;
            let (button_x, y) = canvas.point(60.0 + f32::from(index) * 26.0, 16.0);
            draw_text(
                &label,
                button_x + (25.0 * canvas.scale - width) * 0.5,
                y,
                font_size,
                BLACK,
            );
        }
    }
    draw_tactical_bitmap(
        cache,
        if player_is_alliance {
            art::FIGHTER_SQUADRONS_ALLIANCE
        } else {
            art::FIGHTER_SQUADRONS_EMPIRE
        },
        canvas,
        275.0,
        1.0,
    );
    for index in 0..4_u8 {
        let assigned = groups.fighter_groups[usize::from(index)];
        let active = assigned
            && (groups.selected_fighter_group == Some(index)
                || pressed_control == Some(TacticalHudControl::FighterGroup(index)));
        draw_tactical_bitmap(
            cache,
            if active {
                FIGHTER_GROUP_PRESSED_ART[usize::from(index)]
            } else if assigned {
                FIGHTER_GROUP_NORMAL_ART[usize::from(index)]
            } else {
                art::SQUADRON_UNASSIGNED
            },
            canvas,
            330.0 + f32::from(index) * 26.0,
            2.0,
        );
    }

    let has_selection =
        groups.selected_fighter_group.is_some() || groups.selected_ship_name.is_some();
    if withdraw_confirmation_open {
        draw_tactical_withdraw_confirmation(cache, canvas, pressed_withdraw_confirmation);
    } else if battle_options_open {
        draw_tactical_bitmap(
            cache,
            if player_is_alliance {
                art::BATTLE_OPTIONS_ALLIANCE_PANEL
            } else {
                art::BATTLE_OPTIONS_EMPIRE_PANEL
            },
            canvas,
            481.0,
            27.0,
        );
        for spec in TACTICAL_BATTLE_OPTIONS_CONTROLS {
            if spec.control == TacticalBattleOptionsControl::Withdraw && !withdraw_enabled {
                continue;
            }
            let selected =
                spec.control == TacticalBattleOptionsControl::ObserveToggle && player_observing;
            let pressed = pressed_options == Some(spec.control);
            let resource = if selected || pressed {
                spec.pressed_resource
            } else {
                spec.normal_resource
            };
            draw_tactical_bitmap(cache, resource, canvas, spec.rect.x, spec.rect.y);
        }
    } else if command_panel != TacticalCommandPanel::Display {
        draw_tactical_assignment_panel(
            cache,
            canvas,
            player_is_alliance,
            command_panel,
            groups.selected_fighter_group.is_some(),
            attack_death_star_enabled,
            pressed_command,
        );
    } else {
        let panel_resource = if groups.selected_fighter_group.is_some() {
            1307
        } else if groups.selected_ship_name.is_some() {
            TACTICAL_SELECTED_SHIP_PANEL
        } else {
            1301
        };
        draw_tactical_bitmap(cache, panel_resource, canvas, 481.0, 27.0);
    }
    if !withdraw_confirmation_open
        && !battle_options_open
        && command_panel == TacticalCommandPanel::Display
    {
        if let (Some(name), Some(group)) = (groups.selected_ship_name, groups.selected_task_force) {
            if let Some(resource) = groups.selected_ship_hud {
                draw_tactical_bitmap(
                    cache,
                    resource,
                    canvas,
                    TACTICAL_SELECTED_SHIP_ART_X,
                    TACTICAL_SELECTED_SHIP_ART_Y,
                );
            }
            if let Some((shield, maximum)) = groups.selected_ship_shield {
                draw_tactical_selected_meter(
                    canvas,
                    TACTICAL_SELECTED_SHIELD_METER,
                    tactical_meter_fraction(shield, maximum),
                    Color::new(0.0, 0.0, 1.0, 1.0),
                );
            }
            if let Some((hull, maximum)) = groups.selected_ship_hull {
                draw_tactical_selected_meter(
                    canvas,
                    TACTICAL_SELECTED_HULL_METER,
                    tactical_meter_fraction(hull, maximum),
                    if player_is_alliance {
                        Color::new(0.0, 0.2, 1.0, 1.0)
                    } else {
                        Color::new(1.0, 0.0, 0.0, 1.0)
                    },
                );
            }
            if let Some(subsystems) = groups.selected_ship_subsystems {
                for (index, resource_id) in subsystems.resource_ids().into_iter().enumerate() {
                    draw_tactical_bitmap(
                        cache,
                        resource_id,
                        canvas,
                        TACTICAL_SUBSYSTEM_X[index],
                        TACTICAL_SUBSYSTEM_Y,
                    );
                }
            }
            // TACTICAL 1302 reserves the two blue apertures below the hull
            // portrait for the selected capital ship's personnel and troops.
            // GOKRES supplies the original compact 61x25 assignment images.
            for (index, content) in groups.selected_ship_contents.iter().take(2).enumerate() {
                draw_original_bitmap(
                    cache,
                    DllSource::Gokres,
                    content.resource_id,
                    canvas,
                    491.0 + index as f32 * 63.0,
                    155.0,
                    None,
                );
            }
            let (x, y) = canvas.point(497.0, 48.0);
            draw_text(name, x, y, 11.0 * canvas.scale, WHITE);
            let (x, y) = canvas.point(497.0, 69.0);
            draw_text(
                &format!("Task Force #{}", group + 1),
                x,
                y,
                11.0 * canvas.scale,
                WHITE,
            );
            let (x, y) = canvas.point(497.0, 218.0);
            draw_text(
                groups.selected_order.label(),
                x,
                y,
                10.0 * canvas.scale,
                WHITE,
            );
            let (x, y) = canvas.point(497.0, 230.0);
            draw_text(
                &format!("Tactics: {}", groups.selected_tactic.label()),
                x,
                y,
                9.0 * canvas.scale,
                WHITE,
            );
        } else if let (Some(name), Some(group), Some(hud_resource)) = (
            groups.selected_fighter_name,
            groups.selected_fighter_group,
            groups.selected_fighter_hud,
        ) {
            draw_tactical_bitmap(cache, hud_resource, canvas, 505.0, 76.0);
            let (x, y) = canvas.point(490.0, 45.0);
            draw_text(name, x, y, 10.0 * canvas.scale, WHITE);
            let labels = ["Red Group", "Blue Group", "Green Group", "Gold Group"];
            let (x, y) = canvas.point(490.0, 58.0);
            draw_text(
                labels[usize::from(group.min(3))],
                x,
                y,
                10.0 * canvas.scale,
                WHITE,
            );
            let (x, y) = canvas.point(490.0, 69.0);
            draw_text(
                groups.selected_order.label(),
                x,
                y,
                9.0 * canvas.scale,
                WHITE,
            );
        }
    }
    if !withdraw_confirmation_open
        && !battle_options_open
        && command_panel == TacticalCommandPanel::Display
        && has_selection
    {
        if groups.selected_ship_name.is_some() {
            for (control, normal, pressed_art) in [
                (
                    TacticalCommandControl::PreviousCapital,
                    art::BTN_PREVIOUS_CAPITAL_NORMAL,
                    art::BTN_PREVIOUS_CAPITAL_PRESSED,
                ),
                (
                    TacticalCommandControl::NextCapital,
                    art::BTN_NEXT_CAPITAL_NORMAL,
                    art::BTN_NEXT_CAPITAL_PRESSED,
                ),
            ] {
                draw_tactical_assignment_button(
                    cache,
                    canvas,
                    control,
                    if pressed_command == Some(control) {
                        pressed_art
                    } else {
                        normal
                    },
                );
            }
        }
        for (control, normal, pressed_art) in [
            (
                TacticalCommandControl::OpenMissions,
                art::BTN_MISSIONS_NORMAL,
                art::BTN_MISSIONS_PRESSED,
            ),
            (
                TacticalCommandControl::OpenManeuvers,
                art::BTN_MANEUVERS_TACTICS_NORMAL,
                art::BTN_MANEUVERS_TACTICS_PRESSED,
            ),
        ] {
            draw_tactical_assignment_button(
                cache,
                canvas,
                control,
                if pressed_command == Some(control) {
                    pressed_art
                } else {
                    normal
                },
            );
        }
    }

    for (index, x) in [485.0, 521.0, 560.0, 601.0].into_iter().enumerate() {
        let selected = navigation_sets_visible[index]
            || pressed_control == Some(TacticalHudControl::NavigationSet(index as u8));
        draw_tactical_bitmap(
            cache,
            if selected {
                art::BTN_NAVIGATION_SET_SELECTED[index]
            } else {
                art::BTN_NAVIGATION_SET_NORMAL[index]
            },
            canvas,
            x,
            272.0,
        );
    }
    draw_tactical_bitmap(
        cache,
        if highlight_empire {
            art::HIGHLIGHT_EMPIRE_SHIPS
        } else {
            art::DIM_EMPIRE_SHIPS
        },
        canvas,
        482.0,
        304.0,
    );
    draw_tactical_bitmap(
        cache,
        if highlight_alliance {
            art::HIGHLIGHT_ALLIANCE_SHIPS
        } else {
            art::DIM_ALLIANCE_SHIPS
        },
        canvas,
        517.0,
        304.0,
    );
    draw_tactical_bitmap(
        cache,
        if paused {
            art::BTN_PAUSE_PAUSED
        } else {
            art::BTN_PAUSE_RUNNING
        },
        canvas,
        560.0,
        307.0,
    );
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::BattleOptions) {
            art::BTN_BATTLE_OPTIONS_PRESSED
        } else {
            art::BTN_BATTLE_OPTIONS_NORMAL
        },
        canvas,
        606.0,
        308.0,
    );

    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::ZoomIn) {
            art::BTN_CAMERA_ZOOM_IN_PRESSED
        } else {
            art::BTN_CAMERA_ZOOM_IN_NORMAL
        },
        canvas,
        486.0,
        343.0,
    );
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::ZoomOut) {
            art::BTN_CAMERA_ZOOM_OUT_PRESSED
        } else {
            art::BTN_CAMERA_ZOOM_OUT_NORMAL
        },
        canvas,
        603.0,
        343.0,
    );
    for (id, x, y) in [
        (
            if pressed_control == Some(TacticalHudControl::CameraLeft) {
                art::BTN_CAMERA_LEFT_PRESSED
            } else {
                art::BTN_CAMERA_LEFT_NORMAL
            },
            511.0,
            376.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraRight) {
                art::BTN_CAMERA_RIGHT_PRESSED
            } else {
                art::BTN_CAMERA_RIGHT_NORMAL
            },
            557.0,
            376.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraUp) {
                art::BTN_CAMERA_UP_PRESSED
            } else {
                art::BTN_CAMERA_UP_NORMAL
            },
            537.0,
            344.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraDown) {
                art::BTN_CAMERA_DOWN_PRESSED
            } else {
                art::BTN_CAMERA_DOWN_NORMAL
            },
            537.0,
            412.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraTarget) {
                art::BTN_CAMERA_TARGET_PRESSED
            } else {
                art::BTN_CAMERA_TARGET_NORMAL
            },
            538.0,
            379.0,
        ),
    ] {
        draw_tactical_bitmap(cache, id, canvas, x, y);
    }
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::CameraRecall) {
            art::BTN_CAMERA_RECALL_PRESSED
        } else {
            art::BTN_CAMERA_RECALL_NORMAL
        },
        canvas,
        484.0,
        430.0,
    );
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::CameraMemorize) {
            art::BTN_CAMERA_MEMORIZE_PRESSED
        } else {
            art::BTN_CAMERA_MEMORIZE_NORMAL
        },
        canvas,
        585.0,
        430.0,
    );
}

fn activate_tactical_command(state: &mut TacticalState, control: TacticalCommandControl) {
    match control {
        TacticalCommandControl::PreviousCapital | TacticalCommandControl::NextCapital => {
            let step = if control == TacticalCommandControl::PreviousCapital {
                -1
            } else {
                1
            };
            if let Some((from, to, group)) = state
                .session
                .as_mut()
                .and_then(|session| cycle_selected_capital(session, step))
            {
                macroquad::logging::info!(
                    "[tactical_selection] direction={} task_force={} from={} to={}",
                    if step < 0 { "previous" } else { "next" },
                    group + 1,
                    from,
                    to,
                );
            }
        }
        TacticalCommandControl::OpenManeuvers => {
            if let Some(session) = state.session.as_ref() {
                let (pending_order, pending_tactic) = selected_command_values(session);
                state.command_panel = TacticalCommandPanel::Maneuvers {
                    pending_order,
                    pending_tactic,
                };
            }
        }
        TacticalCommandControl::OpenMissions => {
            if let Some(session) = state.session.as_ref() {
                let (pending_order, _) = selected_command_values(session);
                state.command_panel = TacticalCommandPanel::Missions { pending_order };
            }
        }
        TacticalCommandControl::LeftHook => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = TacticalOrder::LeftHook;
            }
        }
        TacticalCommandControl::RightHook => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = TacticalOrder::RightHook;
            }
        }
        TacticalCommandControl::Hammer => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = TacticalOrder::Hammer;
            }
        }
        TacticalCommandControl::Anvil => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = TacticalOrder::Anvil;
            }
        }
        TacticalCommandControl::HoldPosition => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = TacticalOrder::HoldPosition;
            }
        }
        TacticalCommandControl::ToggleTactic => {
            if let TacticalCommandPanel::Maneuvers { pending_tactic, .. } = &mut state.command_panel
            {
                *pending_tactic = if *pending_tactic == TacticalTactic::StandOff {
                    TacticalTactic::Surround
                } else {
                    TacticalTactic::StandOff
                };
            }
        }
        TacticalCommandControl::AttackCapitalShips => {
            if let TacticalCommandPanel::Missions { pending_order } = &mut state.command_panel {
                *pending_order = TacticalOrder::AttackCapitalShips;
            }
        }
        TacticalCommandControl::AttackFighters => {
            if let TacticalCommandPanel::Missions { pending_order } = &mut state.command_panel {
                *pending_order = TacticalOrder::AttackFighters;
            }
        }
        TacticalCommandControl::Recover => {
            if let TacticalCommandPanel::Missions { pending_order } = &mut state.command_panel {
                *pending_order = TacticalOrder::Recover;
            }
        }
        TacticalCommandControl::AttackDeathStar => {
            if let TacticalCommandPanel::Missions { pending_order } = &mut state.command_panel {
                *pending_order = TacticalOrder::AttackDeathStar;
            }
        }
        TacticalCommandControl::Confirm => {
            let panel = state.command_panel;
            if let Some(session) = state.session.as_mut() {
                let (_, previous_tactic) = selected_command_values(session);
                let (kind, order, tactic) = match panel {
                    TacticalCommandPanel::Maneuvers {
                        pending_order,
                        pending_tactic,
                    } => ("maneuvers", pending_order, Some(pending_tactic)),
                    TacticalCommandPanel::Missions { pending_order } => {
                        ("missions", pending_order, None)
                    }
                    TacticalCommandPanel::Display => return,
                };
                let (capital_members, fighter_members) =
                    assign_selected_command(session, order, tactic);
                if capital_members + fighter_members > 0 {
                    queue_selected_command_voice(
                        session,
                        panel,
                        order,
                        tactic.is_some_and(|value| value != previous_tactic),
                    );
                }
                if order == TacticalOrder::AttackDeathStar {
                    macroquad::logging::info!(
                        "[tactical_death_star] trench_run_launch status={} fighter_members={} group={} remaining_ms={} rng_state={}",
                        if session.trench_run_pending_outcome.is_some() {
                            "launched"
                        } else {
                            "rejected"
                        },
                        fighter_members,
                        session.trench_run_group.map_or(4, |group| group),
                        session.trench_run_remaining_ms.unwrap_or(0.0),
                        session.tactical_rng.state(),
                    );
                }
                macroquad::logging::info!(
                    "[tactical_orders] panel={} event=commit order={} tactic={} capital_members={} fighter_members={}",
                    kind,
                    order.source_code(),
                    tactic.map_or(0, TacticalTactic::source_code),
                    capital_members,
                    fighter_members,
                );
            }
            state.command_panel = TacticalCommandPanel::Display;
        }
        TacticalCommandControl::Cancel => {
            macroquad::logging::info!("[tactical_orders] event=cancel");
            state.command_panel = TacticalCommandPanel::Display;
        }
    }
}

fn activate_tactical_battle_options(
    state: &mut TacticalState,
    control: TacticalBattleOptionsControl,
) -> TacticalAction {
    match control {
        TacticalBattleOptionsControl::Withdraw => {
            state.battle_options_open = false;
            state.battle_options_pressed = None;
            state.withdraw_confirmation_open = true;
            state.withdraw_confirmation_pressed = None;
            macroquad::logging::info!(
                "[tactical_options] command=withdraw status=confirmation_open panel=1310 controls=1113:1114:1115:1116 title=56772 body=56771"
            );
            TacticalAction::None
        }
        TacticalBattleOptionsControl::SimulateRemainder => {
            macroquad::logging::info!("[tactical_options] command=simulate_remainder");
            TacticalAction::AutoResolve
        }
        TacticalBattleOptionsControl::ObserveToggle => {
            state.player_observing = !state.player_observing;
            macroquad::logging::info!(
                "[tactical_options] command={} observing={}",
                if state.player_observing {
                    "observe"
                } else {
                    "take_command"
                },
                state.player_observing
            );
            TacticalAction::None
        }
        TacticalBattleOptionsControl::GameOptions => {
            macroquad::logging::info!("[tactical_options] command=game_options status=routed");
            TacticalAction::OpenGameOptions
        }
        TacticalBattleOptionsControl::Close => {
            state.battle_options_open = false;
            macroquad::logging::info!("[tactical_options] command=close");
            TacticalAction::None
        }
    }
}

fn activate_tactical_withdraw_confirmation(
    state: &mut TacticalState,
    control: TacticalWithdrawConfirmationControl,
) -> TacticalAction {
    state.withdraw_confirmation_open = false;
    state.withdraw_confirmation_pressed = None;
    match control {
        TacticalWithdrawConfirmationControl::Confirm => {
            let hyperdrive_warning = if let Some(session) = state.session.as_mut() {
                session.queue_withdrawal_started_voice();
                session.queue_withdrawal_hyperdrive_warning()
            } else {
                false
            };
            macroquad::logging::info!(
                "[tactical_options] command=withdraw status=withdrawal_started confirmation=accepted hyperdrive_warning={}",
                hyperdrive_warning,
            );
            TacticalAction::WithdrawFromBattle
        }
        TacticalWithdrawConfirmationControl::Cancel => {
            macroquad::logging::info!(
                "[tactical_options] command=withdraw status=confirmation_cancelled"
            );
            TacticalAction::None
        }
    }
}

fn update_tactical_camera_follow(state: &mut TacticalState) -> bool {
    let Some(follow) = state.camera_follow else {
        return false;
    };
    let Some(session) = state.session.as_ref() else {
        state.camera_follow = None;
        return false;
    };
    let (source_positions, fallback_positions, object_id): (Vec<_>, Vec<_>, u32) = match follow {
        TacticalCameraFollow::TaskForce(group) => {
            let members = session
                .ships
                .iter()
                .filter(|ship| {
                    ship.alive
                        && !ship.retreating
                        && ship.is_attacker == session.player_is_attacker
                        && ship.task_force == group
                })
                .collect::<Vec<_>>();
            (
                members.iter().map(|ship| ship.source_position).collect(),
                members.iter().map(|ship| (ship.x, ship.y)).collect(),
                10_001 + u32::from(group),
            )
        }
        TacticalCameraFollow::FighterGroup(group) => {
            let members = session
                .fighters
                .iter()
                .filter(|fighter| {
                    fighter.alive
                        && fighter.squad_count > 0
                        && fighter.is_attacker == session.player_is_attacker
                        && fighter.fighter_group == group
                })
                .collect::<Vec<_>>();
            (
                members
                    .iter()
                    .map(|fighter| fighter.source_position)
                    .collect(),
                members
                    .iter()
                    .map(|fighter| (fighter.x, fighter.y))
                    .collect(),
                10_101 + u32::from(group),
            )
        }
    };
    if source_positions.is_empty() {
        state.camera_follow = None;
        return false;
    }
    let count = source_positions.len() as f32;
    let source = TacticalWorldPosition {
        x: source_positions
            .iter()
            .map(|position| position.x)
            .sum::<f32>()
            / count,
        y: source_positions
            .iter()
            .map(|position| position.y)
            .sum::<f32>()
            / count,
        z: source_positions
            .iter()
            .map(|position| position.z)
            .sum::<f32>()
            / count,
    };
    let fallback_count = fallback_positions.len() as f32;
    let fallback_x = fallback_positions
        .iter()
        .map(|position| position.0)
        .sum::<f32>()
        / fallback_count;
    let fallback_y = fallback_positions
        .iter()
        .map(|position| position.1)
        .sum::<f32>()
        / fallback_count;
    state.camera_x = fallback_x - ARENA_WIDTH * 0.5;
    state.camera_y = fallback_y - ARENA_HEIGHT * 0.5;
    state
        .asset_renderer
        .focus_target(object_id, source.rendered());
    true
}

fn handle_original_tactical_controls(
    state: &mut TacticalState,
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
) -> TacticalAction {
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    if state.withdraw_confirmation_open {
        let hovered = tactical_withdraw_confirmation_control_at(cache, x, y);
        if is_mouse_button_pressed(MouseButton::Left) {
            state.withdraw_confirmation_pressed = hovered;
        }
        if is_mouse_button_released(MouseButton::Left) {
            let captured = state.withdraw_confirmation_pressed.take();
            if let Some(captured) = captured.filter(|captured| Some(*captured) == hovered) {
                return activate_tactical_withdraw_confirmation(state, captured);
            }
        } else if !is_mouse_button_down(MouseButton::Left) {
            state.withdraw_confirmation_pressed = None;
        }
        return TacticalAction::None;
    }
    let death_star_ready = state
        .session
        .as_ref()
        .is_some_and(tactical_death_star_laser_ready);
    if state.death_star_targeting && !death_star_ready {
        state.death_star_targeting = false;
    }
    let death_star_hovered = death_star_ready && tactical_death_star_laser_control_at(cache, x, y);
    if is_mouse_button_pressed(MouseButton::Left) && death_star_hovered {
        state.death_star_laser_pressed = true;
        return TacticalAction::None;
    }
    if is_mouse_button_released(MouseButton::Left) && state.death_star_laser_pressed {
        state.death_star_laser_pressed = false;
        if death_star_hovered {
            state.death_star_targeting = true;
            macroquad::logging::info!(
                "[tactical_death_star] superlaser_targeting status=armed input=right_click"
            );
        }
        return TacticalAction::None;
    }
    if !is_mouse_button_down(MouseButton::Left) {
        state.death_star_laser_pressed = false;
    }
    if state.battle_options_open {
        let withdraw_enabled = state
            .session
            .as_ref()
            .is_some_and(tactical_withdraw_enabled);
        let hovered = tactical_battle_options_control_at(cache, withdraw_enabled, x, y);
        if is_mouse_button_pressed(MouseButton::Left) {
            state.battle_options_pressed = hovered;
        }
        if is_mouse_button_released(MouseButton::Left) {
            let captured = state.battle_options_pressed.take();
            if let Some(captured) = captured.filter(|captured| Some(*captured) == hovered) {
                return activate_tactical_battle_options(state, captured);
            }
        } else if !is_mouse_button_down(MouseButton::Left) {
            state.battle_options_pressed = None;
        }
        return TacticalAction::None;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let (has_selection, selected_fighter, attack_death_star_enabled, player_is_alliance) =
            state
                .session
                .as_ref()
                .map_or((false, false, false, true), |session| {
                    let selected_fighter = session.selected_fighter_group.is_some();
                    let attack_death_star_enabled = attack_death_star_enabled(session);
                    (
                        selected_fighter
                            || session.ships.iter().any(|ship| {
                                ship.alive
                                    && ship.selected
                                    && ship.is_attacker == session.player_is_attacker
                            }),
                        selected_fighter,
                        attack_death_star_enabled,
                        session.player_is_attacker == session.attacker_is_alliance,
                    )
                });
        let command_control = if state.player_observing {
            None
        } else {
            tactical_command_control_at(
                cache,
                state.command_panel,
                has_selection,
                selected_fighter,
                attack_death_star_enabled,
                player_is_alliance,
                x,
                y,
            )
        };
        if let Some(control) = command_control {
            activate_tactical_command(state, control);
        } else {
            match tactical_hud_control_at(cache, x, y) {
                Some(TacticalHudControl::NavigationSet(set)) => {
                    if let Some(visible) = state.navigation_sets_visible.get_mut(usize::from(set)) {
                        *visible = !*visible;
                        state.logged_navigation_scene = None;
                        macroquad::logging::info!(
                            "[tactical_navigation] event=set_visibility set={} visible={}",
                            set + 1,
                            *visible,
                        );
                    }
                }
                Some(TacticalHudControl::TaskForce(group)) => {
                    if let Some(session) = state.session.as_mut() {
                        let control =
                            is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                        let changed = if control {
                            assign_selected_to_task_force(session, group)
                        } else {
                            select_task_force(session, group)
                        };
                        if changed > 0 {
                            state.command_panel = TacticalCommandPanel::Display;
                            macroquad::logging::info!(
                                "[tactical_groups] task_force_{} slot={} members={}",
                                if control { "assigned" } else { "selected" },
                                group + 1,
                                changed,
                            );
                        }
                    }
                }
                Some(TacticalHudControl::FighterGroup(group)) => {
                    if let Some(session) = state.session.as_mut() {
                        if select_fighter_group(session, group) {
                            state.command_panel = TacticalCommandPanel::Display;
                            let members = session
                                .fighters
                                .iter()
                                .filter(|fighter| fighter.selected)
                                .count();
                            macroquad::logging::info!(
                                "[tactical_groups] fighter_group_selected slot={} members={}",
                                group + 1,
                                members,
                            );
                        }
                    }
                }
                Some(TacticalHudControl::Pause) => return TacticalAction::TogglePause,
                Some(TacticalHudControl::ZoomIn) => {
                    state.camera_follow = None;
                    state.zoom = (state.zoom * 1.25).min(2.0);
                    state.asset_renderer.zoom_in();
                }
                Some(TacticalHudControl::ZoomOut) => {
                    state.camera_follow = None;
                    state.zoom = (state.zoom / 1.25).max(0.5);
                    state.asset_renderer.zoom_out();
                }
                Some(TacticalHudControl::CameraTarget) => {
                    state.camera_follow = None;
                    let selected = state.session.as_ref().and_then(|session| {
                        session.selected_ship.and_then(|index| {
                            session.ships.get(index).map(|ship| {
                                (index, ship.x, ship.y, ship.source_position.rendered())
                            })
                        })
                    });
                    if let Some((_index, x, y, _source_position)) = selected {
                        (state.camera_x, state.camera_y) = camera_offset_for_target(x, y);
                        state.asset_renderer.focus_target(
                            u32::try_from(_index).unwrap_or(u32::MAX).saturating_add(1),
                            _source_position,
                        );
                    }
                }
                Some(TacticalHudControl::CameraLeft) => {
                    if state.camera_follow.take().is_some() {
                        macroquad::logging::info!(
                            "[tactical_camera] command=follow_exit reason=manual"
                        );
                    }
                    state.asset_renderer.turn_left();
                }
                Some(TacticalHudControl::CameraRight) => {
                    state.camera_follow = None;
                    state.asset_renderer.turn_right();
                }
                Some(TacticalHudControl::CameraUp) => {
                    state.camera_follow = None;
                    state.asset_renderer.pitch_up();
                }
                Some(TacticalHudControl::CameraDown) => {
                    state.camera_follow = None;
                    state.asset_renderer.pitch_down();
                }
                Some(TacticalHudControl::CameraMemorize) => {
                    state.memorized_camera =
                        state
                            .asset_renderer
                            .camera_state()
                            .map(|source| TacticalCameraBookmark {
                                source,
                                camera_x: state.camera_x,
                                camera_y: state.camera_y,
                                zoom: state.zoom,
                            });
                    macroquad::logging::info!(
                        "[tactical_camera] command=memorize stored={}",
                        state.memorized_camera.is_some(),
                    );
                }
                Some(TacticalHudControl::CameraRecall) => {
                    if let Some(bookmark) = state.memorized_camera.or(state.default_camera) {
                        state.asset_renderer.restore_camera_state(bookmark.source);
                        state.camera_x = bookmark.camera_x;
                        state.camera_y = bookmark.camera_y;
                        state.zoom = bookmark.zoom;
                        state.camera_follow = None;
                        macroquad::logging::info!(
                            "[tactical_camera] command=recall source={}",
                            if state.memorized_camera.is_some() {
                                "memorized"
                            } else {
                                "default"
                            },
                        );
                    }
                }
                Some(TacticalHudControl::HighlightAlliance) => {
                    state.highlight_alliance = !state.highlight_alliance;
                }
                Some(TacticalHudControl::HighlightEmpire) => {
                    state.highlight_empire = !state.highlight_empire;
                }
                Some(TacticalHudControl::BattleOptions) => {
                    state.battle_options_open = true;
                    state.withdraw_confirmation_open = false;
                    state.withdraw_confirmation_pressed = None;
                    state.command_panel = TacticalCommandPanel::Display;
                    macroquad::logging::info!("[tactical_options] command=open");
                }
                None => {}
            }
        }
    }
    let task_force_keys = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
    ];
    for (index, key) in task_force_keys.iter().enumerate() {
        if is_key_pressed(*key) {
            if let Some(session) = state.session.as_mut() {
                if select_task_force(session, u8::try_from(index).unwrap_or(7)) > 0 {
                    state.command_panel = TacticalCommandPanel::Display;
                    let group = u8::try_from(index).unwrap_or(7);
                    state.camera_follow = Some(TacticalCameraFollow::TaskForce(group));
                    macroquad::logging::info!(
                        "[tactical_camera] command=follow kind=task_force group={}",
                        group + 1,
                    );
                }
            }
        }
    }
    let fighter_group_keys = [KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12];
    for (index, key) in fighter_group_keys.iter().enumerate() {
        if is_key_pressed(*key) {
            if let Some(session) = state.session.as_mut() {
                if select_fighter_group(session, u8::try_from(index).unwrap_or(3)) {
                    state.command_panel = TacticalCommandPanel::Display;
                    let group = u8::try_from(index).unwrap_or(3);
                    state.camera_follow = Some(TacticalCameraFollow::FighterGroup(group));
                    macroquad::logging::info!(
                        "[tactical_camera] command=follow kind=fighter_group group={}",
                        group + 1,
                    );
                }
            }
        }
    }
    if is_key_pressed(KeyCode::Equal) {
        state.camera_follow = None;
        state.zoom = (state.zoom * 1.25).min(2.0);
        state.asset_renderer.zoom_in();
    }
    if is_key_pressed(KeyCode::Minus) {
        state.camera_follow = None;
        state.zoom = (state.zoom / 1.25).max(0.5);
        state.asset_renderer.zoom_out();
    }
    if is_key_pressed(KeyCode::Enter) {
        state.camera_follow = None;
        let selected = state.session.as_ref().and_then(|session| {
            session.selected_ship.and_then(|index| {
                session.ships.get(index).map(|ship| {
                    (
                        u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                        ship.x,
                        ship.y,
                        ship.source_position.rendered(),
                    )
                })
            })
        });
        if let Some((object_id, x, y, source)) = selected {
            (state.camera_x, state.camera_y) = camera_offset_for_target(x, y);
            state.asset_renderer.focus_target(object_id, source);
        }
    }
    TacticalAction::None
}

fn camera_offset_for_target(x: f32, y: f32) -> (f32, f32) {
    (x - ARENA_WIDTH * 0.5, y - ARENA_HEIGHT * 0.5)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A unit's column (`FUN_004714e0`): a damaged unit is still operational.
enum BattleResultCondition {
    Operational,
    Destroyed,
}

#[derive(Debug, Clone)]
struct BattleResultEntry {
    name: String,
    condition: BattleResultCondition,
    /// Its `FUN_0042c3b0` picture (`FUN_004724a0`): a craft's or a
    /// regiment's GOKRES portrait, personnel's mini.
    picture: Option<(DllSource, u32)>,
    /// The backdrop under it: a regiment's STRATEGY 11514, personnel's
    /// 11500.
    backdrop: Option<(DllSource, u32)>,
}

#[derive(Debug, Clone, Copy)]
struct BattleResultSkin {
    frame: u32,
    rail: u32,
    close: [u32; 2],
    tabs: [[u32; 2]; 4],
    close_rect: NativeRect,
    tab_rects: [NativeRect; 4],
}

#[derive(Debug, Clone, Copy)]
struct BattleAlertSkin {
    frame: u32,
    scene: u32,
    rail: u32,
    tabs: [[u32; 2]; 4],
    tab_rects: [NativeRect; 4],
    buttons: [[u32; 3]; 3],
}

fn battle_alert_skin(player_is_alliance: bool) -> BattleAlertSkin {
    use resources::strategy as art;
    if player_is_alliance {
        BattleAlertSkin {
            frame: art::BATTLE_ALERT_WINDOW_ALLIANCE,
            scene: art::BATTLE_ALERT_SCENE_ALLIANCE,
            rail: art::BATTLE_RAIL_ALLIANCE,
            tabs: [
                [
                    art::BATTLE_SUMMARY_ALLIANCE_NORMAL,
                    art::BATTLE_SUMMARY_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_ALLIANCE_FORCES_ALLIANCE_NORMAL,
                    art::BATTLE_ALLIANCE_FORCES_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_EMPIRE_FORCES_ALLIANCE_NORMAL,
                    art::BATTLE_EMPIRE_FORCES_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_SYSTEM_ALLIANCE_NORMAL,
                    art::BATTLE_SYSTEM_ALLIANCE_PRESSED,
                ],
            ],
            tab_rects: [
                NativeRect::new(418.0, 21.0, 41.0, 41.0),
                NativeRect::new(418.0, 81.0, 41.0, 41.0),
                NativeRect::new(418.0, 143.0, 41.0, 41.0),
                NativeRect::new(418.0, 205.0, 41.0, 41.0),
            ],
            buttons: [
                [
                    art::BATTLE_ALERT_RETREAT_ALLIANCE_NORMAL,
                    art::BATTLE_ALERT_RETREAT_ALLIANCE_PRESSED,
                    art::BATTLE_ALERT_RETREAT_ALLIANCE_DISABLED,
                ],
                [
                    art::BATTLE_ALERT_SIMULATE_ALLIANCE_NORMAL,
                    art::BATTLE_ALERT_SIMULATE_ALLIANCE_PRESSED,
                    art::BATTLE_ALERT_SIMULATE_ALLIANCE_DISABLED,
                ],
                [
                    art::BATTLE_ALERT_COMMAND_ALLIANCE_NORMAL,
                    art::BATTLE_ALERT_COMMAND_ALLIANCE_PRESSED,
                    art::BATTLE_ALERT_COMMAND_ALLIANCE_DISABLED,
                ],
            ],
        }
    } else {
        BattleAlertSkin {
            frame: art::BATTLE_ALERT_WINDOW_EMPIRE,
            scene: art::BATTLE_ALERT_SCENE_EMPIRE,
            rail: art::BATTLE_RAIL_EMPIRE,
            tabs: [
                [
                    art::BATTLE_SUMMARY_EMPIRE_NORMAL,
                    art::BATTLE_SUMMARY_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_ALLIANCE_FORCES_EMPIRE_NORMAL,
                    art::BATTLE_ALLIANCE_FORCES_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_EMPIRE_FORCES_EMPIRE_NORMAL,
                    art::BATTLE_EMPIRE_FORCES_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_SYSTEM_EMPIRE_NORMAL,
                    art::BATTLE_SYSTEM_EMPIRE_PRESSED,
                ],
            ],
            tab_rects: [
                NativeRect::new(426.0, 17.0, 44.0, 41.0),
                NativeRect::new(426.0, 80.0, 44.0, 41.0),
                NativeRect::new(426.0, 143.0, 44.0, 41.0),
                NativeRect::new(426.0, 206.0, 44.0, 41.0),
            ],
            buttons: [
                [
                    art::BATTLE_ALERT_RETREAT_EMPIRE_NORMAL,
                    art::BATTLE_ALERT_RETREAT_EMPIRE_PRESSED,
                    art::BATTLE_ALERT_RETREAT_EMPIRE_DISABLED,
                ],
                [
                    art::BATTLE_ALERT_SIMULATE_EMPIRE_NORMAL,
                    art::BATTLE_ALERT_SIMULATE_EMPIRE_PRESSED,
                    art::BATTLE_ALERT_SIMULATE_EMPIRE_DISABLED,
                ],
                [
                    art::BATTLE_ALERT_COMMAND_EMPIRE_NORMAL,
                    art::BATTLE_ALERT_COMMAND_EMPIRE_PRESSED,
                    art::BATTLE_ALERT_COMMAND_EMPIRE_DISABLED,
                ],
            ],
        }
    }
}

fn battle_result_skin(player_is_alliance: bool) -> BattleResultSkin {
    use resources::strategy as art;
    if player_is_alliance {
        BattleResultSkin {
            frame: art::BATTLE_WINDOW_ALLIANCE,
            rail: art::BATTLE_RAIL_ALLIANCE,
            close: [
                art::BATTLE_CLOSE_ALLIANCE_NORMAL,
                art::BATTLE_CLOSE_ALLIANCE_PRESSED,
            ],
            tabs: [
                [
                    art::BATTLE_SUMMARY_ALLIANCE_NORMAL,
                    art::BATTLE_SUMMARY_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_ALLIANCE_FORCES_ALLIANCE_NORMAL,
                    art::BATTLE_ALLIANCE_FORCES_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_EMPIRE_FORCES_ALLIANCE_NORMAL,
                    art::BATTLE_EMPIRE_FORCES_ALLIANCE_PRESSED,
                ],
                [
                    art::BATTLE_DESTINATION_ALLIANCE_NORMAL,
                    art::BATTLE_DESTINATION_ALLIANCE_PRESSED,
                ],
            ],
            close_rect: NativeRect::new(423.0, 25.0, 32.0, 31.0),
            tab_rects: [
                NativeRect::new(418.0, 88.0, 41.0, 41.0),
                NativeRect::new(418.0, 142.0, 41.0, 41.0),
                NativeRect::new(418.0, 196.0, 41.0, 41.0),
                NativeRect::new(418.0, 250.0, 41.0, 41.0),
            ],
        }
    } else {
        BattleResultSkin {
            frame: art::BATTLE_WINDOW_EMPIRE,
            rail: art::BATTLE_RAIL_EMPIRE,
            close: [
                art::BATTLE_CLOSE_EMPIRE_NORMAL,
                art::BATTLE_CLOSE_EMPIRE_PRESSED,
            ],
            tabs: [
                [
                    art::BATTLE_SUMMARY_EMPIRE_NORMAL,
                    art::BATTLE_SUMMARY_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_ALLIANCE_FORCES_EMPIRE_NORMAL,
                    art::BATTLE_ALLIANCE_FORCES_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_EMPIRE_FORCES_EMPIRE_NORMAL,
                    art::BATTLE_EMPIRE_FORCES_EMPIRE_PRESSED,
                ],
                [
                    art::BATTLE_DESTINATION_EMPIRE_NORMAL,
                    art::BATTLE_DESTINATION_EMPIRE_PRESSED,
                ],
            ],
            close_rect: NativeRect::new(426.0, 17.0, 44.0, 41.0),
            tab_rects: [
                NativeRect::new(426.0, 89.0, 44.0, 41.0),
                NativeRect::new(426.0, 148.0, 44.0, 41.0),
                NativeRect::new(426.0, 207.0, 44.0, 41.0),
                NativeRect::new(426.0, 266.0, 44.0, 41.0),
            ],
        }
    }
}

impl BattleResultTab {
    const fn index(self) -> usize {
        match self {
            Self::Summary => 0,
            Self::AllianceForces => 1,
            Self::ImperialForces => 2,
            Self::GoDirectlyTo => 3,
        }
    }

    const fn from_index(index: usize) -> Self {
        match index {
            1 => Self::AllianceForces,
            2 => Self::ImperialForces,
            3 => Self::GoDirectlyTo,
            _ => Self::Summary,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::AllianceForces => "alliance_forces",
            Self::ImperialForces => "imperial_forces",
            Self::GoDirectlyTo => "go_directly_to",
        }
    }
}

impl BattleResultCategory {
    const ALL: [Self; 4] = [
        Self::CapitalShips,
        Self::Fighters,
        Self::Troops,
        Self::Personnel,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::CapitalShips => "Capital Ships",
            Self::Fighters => "Fighters",
            Self::Troops => "Troops",
            Self::Personnel => "Personnel",
        }
    }

    /// The category button's label (`FUN_0046f360`: TEXTSTRA 0x1928,
    /// 0x1929, 0x1930, 0x1931), shown over the columns (`FUN_00471200`).
    const fn title(self) -> &'static str {
        match self {
            Self::CapitalShips => "Capital Ships",
            Self::Fighters => "Fighter Squadrons",
            Self::Troops => "Trooper Regiments",
            Self::Personnel => "Personnel",
        }
    }
}

fn winner_is_alliance(session: &BattleSession) -> Option<bool> {
    match session.winner {
        Some(CombatWinner::Attacker) => Some(session.attacker_is_alliance),
        Some(CombatWinner::Defender) => Some(!session.attacker_is_alliance),
        Some(CombatWinner::Draw) | None => None,
    }
}

fn result_fleet_for_faction(session: &BattleSession, alliance: bool) -> FleetKey {
    if session.attacker_is_alliance == alliance {
        session.attacker_fleet
    } else {
        session.defender_fleet
    }
}

fn battle_result_entries(
    session: &BattleSession,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    alliance: bool,
    category: BattleResultCategory,
) -> Vec<BattleResultEntry> {
    // FUN_0042c3b0(.., 1, 1): a craft's or a regiment's portrait, the mini
    // less 0x4000.
    let portrait = |mini: Option<u32>| mini.map(|mini| (DllSource::Gokres, mini - 0x4000));
    let condition = |destroyed: bool| {
        if destroyed {
            BattleResultCondition::Destroyed
        } else {
            BattleResultCondition::Operational
        }
    };
    match category {
        BattleResultCategory::CapitalShips => {
            let mut entries = session
                .ships
                .iter()
                .filter(|ship| ship.identity.is_alliance == alliance)
                .map(|ship| BattleResultEntry {
                    name: ship.name.clone(),
                    condition: condition(!ship.alive || ship.hull_current <= 0),
                    picture: portrait(
                        world
                            .capital_ship_classes
                            .get(ship.class_key)
                            .and_then(|class| crate::panels::fleets::capital_ship_mini_id(class.dat_id)),
                    ),
                    backdrop: None,
                })
                .collect::<Vec<_>>();
            if let Some(death_star) = session
                .death_star
                .filter(|value| value.is_alliance == alliance)
            {
                entries.push(BattleResultEntry {
                    name: "Death Star".into(),
                    condition: condition(death_star.destroyed || death_star.hull <= 0.0),
                    picture: portrait(crate::panels::fleets::capital_ship_mini_id(
                        rebellion_core::ids::DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
                    )),
                    backdrop: None,
                });
            }
            entries
        }
        BattleResultCategory::Fighters => session
            .fighters
            .iter()
            .filter(|fighter| fighter.identity.is_alliance == alliance)
            .map(|fighter| BattleResultEntry {
                name: fighter.name.clone(),
                condition: condition(!fighter.alive || fighter.squad_count == 0),
                picture: portrait(
                    world
                        .fighter_classes
                        .get(fighter.class_key)
                        .and_then(|class| crate::panels::fleets::fighter_mini_id(class.dat_id)),
                ),
                backdrop: None,
            })
            .collect(),
        BattleResultCategory::Troops => {
            let fleet_key = result_fleet_for_faction(session, alliance);
            let surviving_capacity = session
                .ships
                .iter()
                .filter(|ship| ship.identity.is_alliance == alliance && ship.alive)
                .filter_map(|ship| world.capital_ship_classes.get(ship.class_key))
                .map(|class| class.troop_capacity)
                .fold(0_u32, u32::saturating_add) as usize;
            troop_transport
                .cargo(fleet_key)
                .iter()
                .enumerate()
                .map(|(index, troop)| {
                    let unit = world.troops.get(*troop);
                    let strength = unit.map_or(0, |unit| unit.regiment_strength);
                    let class = unit.map(|unit| unit.class_dat_id);
                    BattleResultEntry {
                        // FUN_004f62d0: the regiment's class name.
                        name: class
                            .and_then(|class| world.buildable_classes.get(&class))
                            .map_or_else(|| format!("Regiment {}", index + 1), |value| value.name.clone()),
                        condition: condition(index >= surviving_capacity || strength <= 0),
                        picture: portrait(
                            class.and_then(|class| {
                                crate::system_window::troop_mini(class).map(|(mini, _)| mini)
                            }),
                        ),
                        backdrop: Some((DllSource::Strategy, 11_514)),
                    }
                })
                .collect()
        }
        BattleResultCategory::Personnel => {
            let fleet_key = result_fleet_for_faction(session, alliance);
            world
                .fleets
                .get(fleet_key)
                .into_iter()
                .flat_map(|fleet| fleet.characters.iter())
                .filter_map(|character| world.characters.get(*character))
                .map(|character| BattleResultEntry {
                    name: character.name.clone(),
                    condition: BattleResultCondition::Operational,
                    // A personnel mini (FUN_004724a0 passes 0) on 11500.
                    picture: rebellion_core::pictures::character_mini_resource_id(
                        character.dat_id,
                        character.is_major,
                    )
                    .map(|mini| (DllSource::Gokres, mini)),
                    backdrop: Some((DllSource::Strategy, 11_500)),
                })
                .collect()
        }
    }
}

fn draw_result_text(
    canvas: TacticalCanvas,
    text: &str,
    x: f32,
    baseline_y: f32,
    size: f32,
    color: Color,
) {
    let (screen_x, screen_y) = canvas.point(x, baseline_y);
    draw_text(text, screen_x, screen_y, size * canvas.scale, color);
}

fn draw_result_text_centered(
    canvas: TacticalCanvas,
    text: &str,
    center_x: f32,
    baseline_y: f32,
    size: f32,
    color: Color,
) {
    let font_size = (size * canvas.scale).max(1.0) as u16;
    let dimensions = measure_text(text, None, font_size, 1.0);
    let (screen_x, screen_y) = canvas.point(center_x, baseline_y);
    draw_text(
        text,
        screen_x - dimensions.width * 0.5,
        screen_y,
        f32::from(font_size),
        color,
    );
}

fn draw_result_wrapped_lines(
    canvas: TacticalCanvas,
    lines: &[String],
    x: f32,
    mut baseline_y: f32,
    max_width: f32,
    size: f32,
    color: Color,
) {
    let font_size = (size * canvas.scale).max(1.0) as u16;
    for line in lines {
        let mut current = String::new();
        for word in line.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if !current.is_empty()
                && measure_text(&candidate, None, font_size, 1.0).width > max_width * canvas.scale
            {
                draw_result_text(canvas, &current, x, baseline_y, size, color);
                baseline_y += size + 2.0;
                current = word.to_owned();
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            draw_result_text(canvas, &current, x, baseline_y, size, color);
            baseline_y += size + 2.0;
        }
    }
}

/// The player's TEXTSTRA message texts for the battle report
/// (`set_battle_texts`); empty until the app hands them over.
static BATTLE_TEXTS: std::sync::RwLock<Option<HashMap<u16, String>>> =
    std::sync::RwLock::new(None);

/// Hand the battle report its TEXTSTRA RCDATA texts (`0x7001..0x7016`).
pub fn set_battle_texts(texts: HashMap<u16, String>) {
    if let Ok(mut slot) = BATTLE_TEXTS.write() {
        *slot = Some(texts);
    }
}

fn battle_result_summary_lines(session: &BattleSession, world: &GameWorld) -> Vec<String> {
    let system_side = world
        .systems
        .get(session.system)
        .and_then(|system| system.control.faction())
        .and_then(|faction| match faction {
            rebellion_core::dat::Faction::Alliance => Some(true),
            rebellion_core::dat::Faction::Empire => Some(false),
            rebellion_core::dat::Faction::Neutral => None,
        });
    battle_report_lines(&loaded_battle_texts(), &battle_report(session, system_side))
}

/// The report's fields for the session's viewer; `system_side` is the
/// system's side after the battle.
fn battle_report(session: &BattleSession, system_side: Option<bool>) -> BattleReport<'_> {
    // +0x54: the side whose fleet withdrew (FUN_0049b060, +0x78 >> 2).
    let withdrew = |alliance: bool| {
        session.ships.iter().any(|ship| {
            ship.identity.is_alliance == alliance && ship.retreated
        })
    };
    let withdrawn = if withdrew(true) {
        Some(true)
    } else if withdrew(false) {
        Some(false)
    } else {
        None
    };
    BattleReport {
        system_name: &session.system_name,
        viewer_alliance: player_is_alliance(session),
        winner_alliance: winner_is_alliance(session),
        withdrawn,
        system_side,
    }
}

fn loaded_battle_texts() -> HashMap<u16, String> {
    BATTLE_TEXTS
        .read()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default()
}

/// What the battle report reads (`FUN_0049ac00`'s object).
struct BattleReport<'a> {
    system_name: &'a str,
    /// `+0x58`: the viewer's side.
    viewer_alliance: bool,
    /// `+0x50`: the winner, `None` for none.
    winner_alliance: Option<bool>,
    /// `+0x54`: the side whose fleet withdrew, `None` for none.
    withdrawn: Option<bool>,
    /// The system's side bits after the battle; `None` neutral.
    system_side: Option<bool>,
}

/// The battle report's headline and body (`FUN_0049b0f0`): no winner
/// (`FUN_0049b150`), the viewer's win (`FUN_0049b330`) or loss
/// (`FUN_0049b4d0`), each sentence a TEXTSTRA RCDATA record appended to the
/// body. port: the withdrawn fleet's destination (`+0x48`) is decided after
/// the report, so "has withdrawn to {2:1}" (`0x700d`) reads "has withdrawn"
/// (`0x700c`); the blockade-before side (`+0x5c`, `0x7015` / `0x7016`) is not
/// kept. Missing texts fall back to the port's English.
fn battle_report_lines(texts: &HashMap<u16, String>, report: &BattleReport<'_>) -> Vec<String> {
    use crate::story_messages::{fill_message_text, side_parameter, MessageParameter};
    let viewer = side_parameter(report.viewer_alliance, texts);
    let enemy = side_parameter(!report.viewer_alliance, texts);
    // A system's kinds 1, 2 and 4 read its name (FUN_004f6030).
    let system = MessageParameter::Unit {
        name: report.system_name,
        class: "",
    };
    let say = |id: u16, parameters: &[MessageParameter<'_>]| {
        texts
            .get(&id)
            .map(|template| fill_message_text(template, parameters))
    };
    let side_word = |alliance: bool| if alliance { "Alliance" } else { "Imperial" };
    let (headline, body): (Option<String>, Vec<Option<String>>) = match report.winner_alliance {
        None => {
            let ending = match report.withdrawn {
                None => vec![say(0x700f, &[viewer, enemy])],
                Some(side) if side == report.viewer_alliance => {
                    vec![say(0x700c, &[viewer]), say(0x700b, &[enemy])]
                }
                Some(_) => vec![say(0x700b, &[viewer]), say(0x700c, &[enemy])],
            };
            (
                say(0x7004, &[system]),
                std::iter::once(say(0x700e, &[viewer, enemy])).chain(ending).collect(),
            )
        }
        Some(winner) if winner == report.viewer_alliance => {
            let first = match report.system_side {
                None => say(0x7007, &[system, enemy]),
                Some(side) if side == report.viewer_alliance => say(0x7005, &[system, enemy]),
                Some(_) => say(0x7006, &[system, viewer]),
            };
            let ending = if report.withdrawn.is_some() {
                say(0x700c, &[enemy])
            } else {
                say(0x700b, &[enemy])
            };
            (say(0x7002, &[viewer]), vec![first, ending])
        }
        Some(_) => {
            let first = match report.system_side {
                None => say(0x700a, &[system, enemy]),
                Some(side) if side == report.viewer_alliance => say(0x7008, &[system, enemy]),
                Some(_) => say(0x7009, &[system, viewer]),
            };
            let ending = if report.withdrawn.is_some() {
                say(0x700c, &[viewer])
            } else {
                say(0x700b, &[viewer])
            };
            (say(0x7003, &[viewer]), vec![first, ending])
        }
    };
    match (headline, body.into_iter().collect::<Option<Vec<String>>>()) {
        (Some(headline), Some(body)) => vec![
            headline.trim_end().to_owned(),
            body.concat().trim_end().to_owned(),
        ],
        _ => {
            // port: the texts are missing (a test or an incomplete pack).
            let Some(winner) = report.winner_alliance else {
                return vec![
                    "Neither fleet is victorious.".into(),
                    "Both forces have been destroyed or withdrawn.".into(),
                ];
            };
            vec![
                format!("The {} fleet is victorious.", side_word(winner)),
                format!(
                    "{} is now under blockade by {} forces.",
                    report.system_name,
                    side_word(winner)
                ),
                if report.withdrawn.is_some() {
                    format!("The {} fleet has withdrawn from the system.", side_word(!winner))
                } else {
                    format!("The {} fleet has been completely destroyed.", side_word(!winner))
                },
            ]
        }
    }
}

/// The Display page's message line (`+0xac`) and instruction (`+0xb8`),
/// and whether the viewer's fleet gets a button: the viewer's win reads
/// `0x7012` and `0x7013` (`FUN_0049b330`); the viewer's withdrawal reads
/// `0x7011` and `0x7013`; otherwise `0x7010` and `0x7014`, the fleet gone
/// (`FUN_0049b150`, `FUN_0049b4d0`). port: the withdrawn fleet's
/// destination (`+0x48`) is decided after the report, so `0x7011` drops
/// its " to {3:1}", as the body reads `0x700c` for `0x700d`.
fn battle_display_lines(
    texts: &HashMap<u16, String>,
    report: &BattleReport<'_>,
) -> ([String; 2], bool) {
    use crate::story_messages::{fill_message_text, side_parameter, MessageParameter};
    let viewer = side_parameter(report.viewer_alliance, texts);
    let system = MessageParameter::Unit {
        name: report.system_name,
        class: "",
    };
    let side = if report.viewer_alliance {
        "Alliance"
    } else {
        "Imperial"
    };
    let (message, instruction, fleet, fallback) =
        if report.winner_alliance == Some(report.viewer_alliance) {
            (
                0x7012,
                0x7013,
                true,
                format!(
                    "The {side} fleet is victorious. The world {} is now under {side} blockade.",
                    report.system_name
                ),
            )
        } else if report.withdrawn == Some(report.viewer_alliance) {
            (
                0x7011,
                0x7013,
                true,
                format!(
                    "The {side} fleet was defeated at {}. Your fleet has withdrawn.",
                    report.system_name
                ),
            )
        } else {
            (
                0x7010,
                0x7014,
                false,
                format!(
                    "The {side} fleet was defeated at {}. Your fleet has been destroyed.",
                    report.system_name
                ),
            )
        };
    let message = texts.get(&message).map_or(fallback, |template| {
        fill_message_text(&template.replace(" to {3:1}", ""), &[viewer, system])
            .trim_end()
            .to_owned()
    });
    let instruction = texts.get(&instruction).map_or_else(
        || {
            if fleet {
                "Select one of the following buttons to close this display and go directly to..."
            } else {
                "Select the following button to close this display and go directly to..."
            }
            .to_owned()
        },
        |text| text.trim_end().to_owned(),
    );
    ([message, instruction], fleet)
}

/// A Display page button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplayButton {
    System,
    Fleet,
}

/// The Battle Summary's Display page (`FUN_0046f360`): the message line
/// at (24, 40) and the instruction at (24, 130), 328 wide, word-wrapped,
/// font 6 and font 7, in the viewer's colour; the system button `0xcb`
/// at (29, 182), 169 by 96, STRATEGY 10764 (`0x2a0c`); the fleet button
/// `0xca` at (229, 182), 10765 / 10766 by the viewer's side, only while
/// the viewer has a fleet there (`FUN_004f36a0`). Both press to 10767 /
/// 10768 by the viewer's side. Under each, at y 278, 169 wide, centred,
/// white, font 4: the system's name and "Fleet" (`0x1942`).
fn battle_display_page(
    system_name: &str,
    lines: &[String; 2],
    viewer_alliance: bool,
    fleet: bool,
    pressed: Option<DisplayButton>,
    text_color: Color,
) -> Vec<PageOp> {
    use resources::strategy as art;
    let mut ops = vec![PageOp::Bitmap {
        source: DllSource::Strategy,
        id: art::BATTLE_DESTINATION_BACKGROUND,
        x: BATTLE_RESULTS_SCENE_X,
        y: BATTLE_RESULTS_SCENE_Y,
    }];
    for (line, top) in lines.iter().zip([40.0, 130.0]) {
        ops.push(PageOp::Wrapped {
            lines: vec![line.clone()],
            x: 24.0,
            baseline: top + 12.0,
            width: 328.0,
            size: 12.0,
            color: text_color,
        });
    }
    let pressed_art = if viewer_alliance {
        art::BATTLE_DESTINATION_ALLIANCE_SELECTED
    } else {
        art::BATTLE_DESTINATION_EMPIRE_SELECTED
    };
    let fleet_art = if viewer_alliance {
        art::BATTLE_DESTINATION_ALLIANCE_FLEET
    } else {
        art::BATTLE_DESTINATION_EMPIRE_FLEET
    };
    let mut buttons = vec![(
        DisplayButton::System,
        29.0,
        art::BATTLE_DESTINATION_SYSTEM,
        system_name,
    )];
    if fleet {
        buttons.push((DisplayButton::Fleet, 229.0, fleet_art, "Fleet"));
    }
    for (button, x, id, label) in buttons {
        ops.push(PageOp::Bitmap {
            source: DllSource::Strategy,
            id: if pressed == Some(button) {
                pressed_art
            } else {
                id
            },
            x,
            y: 182.0,
        });
        ops.push(PageOp::Text {
            text: label.to_owned(),
            x: x + 84.5,
            baseline: 278.0 + 14.0,
            size: 14.0,
            color: WHITE,
            centered: true,
        });
    }
    ops
}

/// The Display page's button under the mouse.
fn display_button_at(x: f32, y: f32, fleet: bool) -> Option<DisplayButton> {
    let inside =
        |left: f32| (left..left + 169.0).contains(&x) && (182.0..182.0 + 96.0).contains(&y);
    if inside(29.0) {
        Some(DisplayButton::System)
    } else if fleet && inside(229.0) {
        Some(DisplayButton::Fleet)
    } else {
        None
    }
}

fn draw_battle_result_summary(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    session: &BattleSession,
    world: &GameWorld,
    text_color: Color,
) {
    let ops = battle_summary_page(
        &session.system_name,
        winner_is_alliance(session),
        &battle_result_summary_lines(session, world),
        text_color,
    );
    draw_page_ops(cache, canvas, ops);
}

/// The Battle Summary's summary page (`FUN_0046f360`): the scene; the title
/// at (12, 18), 400 wide, centred, font `0xb`; the outcome's first line at
/// (12, 225), 400 wide, centred, font `0xb`; the rest at (24, 255), 352
/// wide, word-wrapped, font 6, over a black shadow at (25, 256). The text
/// takes the viewer's colour (red for side 1, green for side 2). The
/// capture guide-059 shows the same layout.
fn battle_summary_page(
    system_name: &str,
    winner_alliance: Option<bool>,
    lines: &[String],
    text_color: Color,
) -> Vec<PageOp> {
    use resources::strategy as art;
    let scene = match winner_alliance {
        Some(true) => art::EVENT_BATTLE_ALLIANCE_VICTORY,
        Some(false) => art::EVENT_BATTLE_EMPIRE_VICTORY,
        None => art::EVENT_BATTLE_MUTUAL_DESTRUCTION,
    };
    let mut ops = vec![
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: scene,
            x: BATTLE_RESULTS_SCENE_X,
            y: BATTLE_RESULTS_SCENE_Y,
        },
        PageOp::Text {
            text: format!("Battle at {system_name}"),
            x: 212.0,
            baseline: 18.0 + 16.0,
            size: 18.0,
            color: text_color,
            centered: true,
        },
    ];
    if let Some((headline, rest)) = lines.split_first() {
        ops.push(PageOp::Text {
            text: headline.clone(),
            x: 212.0,
            baseline: 225.0 + 16.0,
            size: 18.0,
            color: text_color,
            centered: true,
        });
        for (x, top, color) in [(25.0, 256.0, BLACK), (24.0, 255.0, text_color)] {
            ops.push(PageOp::Wrapped {
                lines: rest.to_vec(),
                x,
                baseline: top + 12.0,
                width: 352.0,
                size: 12.0,
                color,
            });
        }
    }
    ops
}

/// The Battle Summary's forces page (`FUN_0046f360` builds it,
/// `FUN_004714e0` fills the list, `FUN_00471200` paints its labels): the
/// side's rail label at (12, 40), 400 wide, centred, red for the Alliance
/// and green for the Empire; "Filters" at (86, 70); the category bar at
/// (130, 59); the category's label at (37, 99); the column heads at y 118;
/// the list at (37, 136), 347 by 166, of 165 by 70 cells in two columns
/// (Operational, Destroyed), or 110 by 50 in three for Personnel
/// (Survivors, Captured, Killed). The page art is the two- or
/// three-column bitmap (STRATEGY 10761 / 10762).
#[expect(
    clippy::too_many_arguments,
    reason = "The page reads the battle, the world and the viewer's art."
)]
fn draw_battle_result_force_details(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    session: &BattleSession,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    alliance: bool,
    category: BattleResultCategory,
    player_alliance: bool,
    text_color: Color,
) {
    let entries = battle_result_entries(session, world, troop_transport, alliance, category);
    let ops = battle_force_page(
        &session.system_name,
        &entries,
        alliance,
        category,
        player_alliance,
        text_color,
        |source, id| cache.original_resource_size(source, id),
    );
    draw_page_ops(cache, canvas, ops);
}

/// Draw a results page's steps in the Battle Results window.
fn draw_page_ops(cache: &mut BmpCache, canvas: TacticalCanvas, ops: Vec<PageOp>) {
    draw_page_ops_at(
        cache,
        canvas,
        ops,
        (BATTLE_RESULTS_WINDOW_X, BATTLE_RESULTS_WINDOW_Y),
    );
}

/// Draw a page's steps in a window whose top left is `origin`.
fn draw_page_ops_at(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    ops: Vec<PageOp>,
    (origin_x, origin_y): (f32, f32),
) {
    for op in ops {
        match op {
            PageOp::Bitmap { source, id, x, y } => {
                draw_original_bitmap(cache, source, id, canvas, origin_x + x, origin_y + y, None)
            }
            PageOp::Text {
                text,
                x,
                baseline,
                size,
                color,
                centered,
            } => {
                let (x, y) = (origin_x + x, origin_y + baseline);
                if centered {
                    draw_result_text_centered(canvas, &text, x, y, size, color);
                } else {
                    draw_result_text(canvas, &text, x, y, size, color);
                }
            }
            PageOp::Wrapped {
                lines,
                x,
                baseline,
                width,
                size,
                color,
            } => draw_result_wrapped_lines(
                canvas,
                &lines,
                origin_x + x,
                origin_y + baseline,
                width,
                size,
                color,
            ),
        }
    }
}

/// One drawing step of a results page, in window coordinates.
#[derive(Debug, Clone, PartialEq)]
enum PageOp {
    Bitmap {
        source: DllSource,
        id: u32,
        x: f32,
        y: f32,
    },
    /// Text at its baseline; `centered` on `x`, else from it.
    Text {
        text: String,
        x: f32,
        baseline: f32,
        size: f32,
        color: Color,
        centered: bool,
    },
    /// Lines word-wrapped at `width` from `x`, the first at `baseline`.
    Wrapped {
        lines: Vec<String>,
        x: f32,
        baseline: f32,
        width: f32,
        size: f32,
        color: Color,
    },
}

/// The Battle Summary's forces page as drawing steps (see
/// `draw_battle_result_force_details` for the sources); `size_of` gives a
/// bitmap's size for centring the pictures.
fn battle_force_page(
    system_name: &str,
    entries: &[BattleResultEntry],
    alliance: bool,
    category: BattleResultCategory,
    player_alliance: bool,
    text_color: Color,
    mut size_of: impl FnMut(DllSource, u32) -> Option<[usize; 2]>,
) -> Vec<PageOp> {
    use resources::strategy as art;
    let text = |text: &str, x: f32, top: f32, size: f32, color: Color, centered: bool| {
        PageOp::Text {
            text: text.to_owned(),
            x,
            baseline: top + size,
            size,
            color,
            centered,
        }
    };
    let personnel = category == BattleResultCategory::Personnel;
    let mut ops = vec![
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: if personnel {
                art::BATTLE_FORCE_DETAIL_THREE_COLUMN
            } else {
                art::BATTLE_FORCE_DETAIL_TWO_COLUMN
            },
            x: BATTLE_RESULTS_SCENE_X,
            y: BATTLE_RESULTS_SCENE_Y,
        },
        text(&format!("Battle at {system_name}"), 212.0, 14.0, 18.0, text_color, true),
    ];
    // FUN_00471200: rail 0x70 (Alliance) red, 0x71 (Empire) green.
    let (label, side_color) = if alliance {
        ("Alliance Forces", Color::from_rgba(255, 0, 0, 255))
    } else {
        ("Imperial Forces", Color::from_rgba(0, 255, 0, 255))
    };
    ops.push(text(label, 212.0, 40.0, 14.0, side_color, true));
    ops.push(text("Filters", 86.0, 70.0, 10.0, WHITE, false));
    for (index, item) in BattleResultCategory::ALL.into_iter().enumerate() {
        let (normal, pressed) = battle_category_art(item, player_alliance);
        let rect = battle_category_rect(index);
        ops.push(PageOp::Bitmap {
            source: DllSource::Strategy,
            id: if item == category { pressed } else { normal },
            x: rect.x,
            y: rect.y,
        });
    }
    ops.push(text(category.title(), 37.0, 99.0, 10.0, WHITE, false));
    let (heads, width): (&[(f32, &str)], f32) = if personnel {
        (
            &[(37.0, "Survivors"), (149.0, "Captured"), (260.0, "Killed")],
            110.0,
        )
    } else {
        (&[(37.0, "Operational"), (204.0, "Destroyed")], 165.0)
    };
    for &(x, head) in heads {
        ops.push(text(head, x + width / 2.0, 118.0, 10.0, WHITE, true));
    }

    let columns = battle_result_columns(entries, personnel);
    let (cell_w, cell_h) = if personnel { (110.0, 50.0) } else { (165.0, 70.0) };
    let rows = (BATTLE_RESULT_LIST_HEIGHT / cell_h) as usize;
    let empty = battle_result_empty_labels(&columns);
    for (column, units) in columns.iter().enumerate() {
        let left = 37.0 + column as f32 * cell_w;
        if let Some(label) = empty[column] {
            ops.push(text(label, left + cell_w / 2.0, 140.0, 10.0, WHITE, true));
        }
        for (row, entry) in units.iter().take(rows).enumerate() {
            let top = 136.0 + row as f32 * cell_h;
            // FUN_004724a0: the picture centred over the name.
            if let Some(picture) = entry.picture {
                for (source, id) in entry.backdrop.into_iter().chain(std::iter::once(picture)) {
                    if let Some([w, h]) = size_of(source, id) {
                        ops.push(PageOp::Bitmap {
                            source,
                            id,
                            x: left + ((cell_w - w as f32) / 2.0).floor(),
                            y: top + ((cell_h - BATTLE_RESULT_NAME_HEIGHT - h as f32) / 2.0).floor(),
                        });
                    }
                }
            }
            ops.push(text(
                &entry.name,
                left + cell_w / 2.0,
                top + cell_h - BATTLE_RESULT_NAME_HEIGHT,
                9.0,
                WHITE,
                true,
            ));
        }
    }
    ops
}

/// The height a row keeps under its picture for the name.
const BATTLE_RESULT_NAME_HEIGHT: f32 = 14.0;

/// The list's height (`FUN_00607ea0` at (37, 136), 347 by 166).
const BATTLE_RESULT_LIST_HEIGHT: f32 = 166.0;

/// A category button's rectangle in the window: the bar at (130, 59), four
/// 49 by 41 buttons (`FUN_0046f360`).
fn battle_category_rect(index: usize) -> NativeRect {
    NativeRect::new(130.0 + index as f32 * 49.0, 59.0, 49.0, 41.0)
}

/// A category button's art (normal, pressed) by the viewer's side
/// (`FUN_0046f360`).
const fn battle_category_art(category: BattleResultCategory, alliance: bool) -> (u32, u32) {
    match (category, alliance) {
        (BattleResultCategory::CapitalShips, true) => (0x286c, 0x286b),
        (BattleResultCategory::CapitalShips, false) => (0x2878, 0x2877),
        (BattleResultCategory::Fighters, true) => (0x2872, 0x2871),
        (BattleResultCategory::Fighters, false) => (0x287c, 0x287b),
        (BattleResultCategory::Troops, true) => (0x2870, 0x286f),
        (BattleResultCategory::Troops, false) => (0x287a, 0x2879),
        (BattleResultCategory::Personnel, true) => (0x286a, 0x2869),
        (BattleResultCategory::Personnel, false) => (0x2876, 0x2875),
    }
}

/// The columns `FUN_004714e0` fills: Operational and Destroyed, or for
/// Personnel Survivors, Captured and Killed. port: the battle captures
/// and kills no personnel, so they are all survivors.
fn battle_result_columns(
    entries: &[BattleResultEntry],
    personnel: bool,
) -> Vec<Vec<&BattleResultEntry>> {
    let of = |condition| {
        entries
            .iter()
            .filter(|entry| entry.condition == condition)
            .collect::<Vec<_>>()
    };
    let operational = of(BattleResultCondition::Operational);
    let destroyed = of(BattleResultCondition::Destroyed);
    if personnel {
        vec![operational, Vec::new(), destroyed]
    } else {
        vec![operational, destroyed]
    }
}

/// `FUN_004714e0`'s lines for empty columns: the first reads "None"
/// (`0x8526`) when every column is empty and "No Survivors" (`0x1940`)
/// otherwise; the middle "None"; the last "No Casualties" (`0x1941`).
fn battle_result_empty_labels(columns: &[Vec<&BattleResultEntry>]) -> Vec<Option<&'static str>> {
    let all_empty = columns.iter().all(Vec::is_empty);
    let last = columns.len() - 1;
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            column.is_empty().then_some(if index == last {
                "No Casualties"
            } else if index == 0 && !all_empty {
                "No Survivors"
            } else {
                "None"
            })
        })
        .collect()
}

fn draw_battle_result_destinations(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    session: &BattleSession,
    player_alliance: bool,
    mouse: (f32, f32),
    mouse_down: bool,
    mouse_released: bool,
    text_color: Color,
) -> TacticalAction {
    let (lines, fleet) =
        battle_display_lines(&loaded_battle_texts(), &battle_report(session, None));
    let hovered = display_button_at(
        mouse.0 - BATTLE_RESULTS_WINDOW_X,
        mouse.1 - BATTLE_RESULTS_WINDOW_Y,
        fleet,
    );
    let ops = battle_display_page(
        &session.system_name,
        &lines,
        player_alliance,
        fleet,
        hovered.filter(|_| mouse_down),
        text_color,
    );
    draw_page_ops(cache, canvas, ops);

    match hovered.filter(|_| mouse_released) {
        Some(DisplayButton::System) => {
            macroquad::logging::info!("[tactical-results] destination=system");
            TacticalAction::OpenBattleSystem(session.system)
        }
        Some(DisplayButton::Fleet) => {
            macroquad::logging::info!("[tactical-results] destination=fleet");
            TacticalAction::OpenBattleFleet(result_fleet_for_faction(session, player_alliance))
        }
        None => TacticalAction::None,
    }
}

fn battle_alert_retreat_enabled(session: &BattleSession) -> bool {
    session.ships.iter().any(|ship| {
        ship.alive
            && ship.is_attacker == session.player_is_attacker
            && ship.subsystem_capacity.hyperdrive > 0
            && ship.subsystem_condition.hyperdrive > 0
    })
}

fn draw_battle_alert_text(
    state: &TacticalState,
    cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    canvas: TacticalCanvas,
    text_color: Color,
) {
    let session = state.session.as_ref().expect("active Battle Alert session");
    let player_alliance = player_is_alliance(session);
    let (system_side, last_arrival) = battle_alert_sides(world, session.system);
    let [title, summary] = battle_alert_lines(
        &loaded_battle_texts(),
        player_alliance,
        &session.system_name,
        system_side,
        last_arrival,
    );
    draw_result_text_centered(
        canvas,
        &title,
        BATTLE_ALERT_WINDOW_X + 212.0,
        BATTLE_ALERT_WINDOW_Y + 40.0,
        18.0,
        text_color,
    );
    let lines = match state.battle_alert_tab {
        BattleAlertTab::Summary => {
            // FUN_0044f860: the summary over a black shadow one pixel down
            // and right.
            draw_result_wrapped_lines(
                canvas,
                std::slice::from_ref(&summary),
                BATTLE_ALERT_WINDOW_X + 39.0,
                BATTLE_ALERT_WINDOW_Y + 226.0,
                340.0,
                14.0,
                BLACK,
            );
            vec![summary]
        }
        tab => {
            // FUN_00450660: the other tabs show the scene with the title
            // only (surface 0x33) and the list.
            let Some(content) = BattleAlertList::of(tab) else {
                return;
            };
            let rows = battle_alert_rows(world, troop_transport, session.system, content);
            draw_page_ops_at(
                cache,
                canvas,
                battle_alert_list(&rows),
                (BATTLE_ALERT_WINDOW_X, BATTLE_ALERT_WINDOW_Y),
            );
            return;
        }
    };
    draw_result_wrapped_lines(
        canvas,
        &lines,
        BATTLE_ALERT_WINDOW_X + 38.0,
        BATTLE_ALERT_WINDOW_Y + 225.0,
        340.0,
        14.0,
        text_color,
    );
}

/// What a Battle Alert list tab holds (`FUN_00450770`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleAlertList {
    /// `0x1923` / `0x1924`: one side's forces at the system.
    Forces { alliance: bool },
    /// `0x1925`: what is on the planet.
    SystemAssets,
}

impl BattleAlertList {
    /// The list a tab fills (`FUN_00450660`: tabs 2, 3 and 4 call
    /// `FUN_00450770` with 1, 2 and 0); the summary tab has none.
    const fn of(tab: BattleAlertTab) -> Option<Self> {
        match tab {
            BattleAlertTab::Summary => None,
            BattleAlertTab::AllianceForces => Some(Self::Forces { alliance: true }),
            BattleAlertTab::ImperialForces => Some(Self::Forces { alliance: false }),
            BattleAlertTab::System => Some(Self::SystemAssets),
        }
    }
}

/// A list row's font: `0xb` for the tab's heading, 5 for a fleet, a
/// capital ship or the system, 4 for the units under them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlertFont {
    Heading,
    Bold,
    Plain,
}

/// One row of a Battle Alert list (`FUN_00450e10`): a text row is centred;
/// a picture row draws the object's mini (its layers bottom to top) at the
/// row's left, 20 further for a unit aboard (`param_4`), and the text at
/// 70, or 90.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AlertRow {
    text: String,
    font: AlertFont,
    picture: Vec<(DllSource, u32)>,
    indent: bool,
}

/// The rows `FUN_00450770` fills. A forces tab: the heading; each of the
/// side's fleets at the system (`FUN_004ffef0`), its name, then each
/// capital ship and the units aboard (`FUN_005039d0` squadrons,
/// `FUN_00504c40` regiments, `FUN_00536da0` personnel), indented; then,
/// when the side has squadrons on the planet (`FUN_00503a50`), the
/// system's name and those squadrons. System Assets: the heading, then the
/// facilities (`FUN_00539d70`), the regiments (`FUN_00504c40`) and the
/// personnel (`FUN_00536da0`) on the planet, of either side.
///
/// port: a fleet keeps its squadrons, regiments and personnel, not each
/// ship, so they follow the fleet's ships. Facilities go by kind
/// (defenses, manufacturing, production) in the system's order.
fn battle_alert_rows(
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    system: SystemKey,
    list: BattleAlertList,
) -> Vec<AlertRow> {
    use crate::fleet_window::{mini_marks, MiniObject};
    use crate::system_window::{
        defense_facility_mini, manufacturing_facility_mini, production_facility_mini,
        special_force_mini, troop_mini,
    };
    let text = |text: String, font| AlertRow {
        text,
        font,
        picture: Vec::new(),
        indent: false,
    };
    // FUN_0042c3b0(.., 0, 1): the mini over its backdrop and marks.
    let row = |text: String, font, object, mini: Option<u32>, damaged, indent| AlertRow {
        text,
        font,
        picture: mini
            .map(|mini| {
                let mut layers = mini_marks(object, mini, false, damaged);
                layers.push((DllSource::Gokres, mini));
                layers
            })
            .unwrap_or_default(),
        indent,
    };
    let Some(place) = world.systems.get(system) else {
        return Vec::new();
    };
    let squadron = |entry: &rebellion_core::world::FighterEntry, indent| {
        let class = world.fighter_classes.get(entry.class)?;
        Some((0..entry.count).map(move |_| {
            row(
                class.name.clone(),
                AlertFont::Plain,
                MiniObject::Craft,
                crate::panels::fleets::fighter_mini_id(class.dat_id),
                false,
                indent,
            )
        }))
    };
    let regiment = |key: rebellion_core::ids::TroopKey, indent| {
        let troop = world.troops.get(key)?;
        let (mini, label) = troop_mini(troop.class_dat_id)?;
        Some(row(
            label.to_owned(),
            AlertFont::Plain,
            MiniObject::Regiment(troop.class_dat_id),
            Some(mini),
            false,
            indent,
        ))
    };
    let character = |character: &rebellion_core::world::Character, indent| {
        row(
            character.name.clone(),
            AlertFont::Plain,
            MiniObject::Character,
            crate::system_window::character_mini_resource_id(character.dat_id, character.is_major),
            // FUN_004f7800: an injured character is damaged.
            character.injury > 0,
            indent,
        )
    };
    let mut rows = Vec::new();
    match list {
        BattleAlertList::Forces { alliance } => {
            rows.push(text(
                if alliance {
                    "Alliance Forces"
                } else {
                    "Imperial Forces"
                }
                .to_owned(),
                AlertFont::Heading,
            ));
            for &key in &place.fleets {
                let Some(fleet) = world
                    .fleets
                    .get(key)
                    .filter(|fleet| fleet.is_alliance == alliance)
                else {
                    continue;
                };
                rows.push(text(
                    world.fleet_name(key).unwrap_or_default().to_owned(),
                    AlertFont::Bold,
                ));
                for (index, ship) in fleet.capital_ships.iter().enumerate() {
                    let Some(class) = world.capital_ship_classes.get(ship.class) else {
                        continue;
                    };
                    rows.push(row(
                        world
                            .ship_name(key, index)
                            .unwrap_or(&class.name)
                            .to_owned(),
                        AlertFont::Bold,
                        MiniObject::Craft,
                        crate::panels::fleets::capital_ship_mini_id(class.dat_id),
                        crate::status_rows::damaged(ship, class),
                        false,
                    ));
                }
                rows.extend(
                    fleet
                        .fighters
                        .iter()
                        .filter_map(|entry| squadron(entry, true))
                        .flatten(),
                );
                rows.extend(
                    troop_transport
                        .cargo(key)
                        .iter()
                        .filter_map(|&troop| regiment(troop, true)),
                );
                rows.extend(
                    fleet
                        .characters
                        .iter()
                        .filter_map(|&key| world.characters.get(key))
                        .map(|value| character(value, true)),
                );
            }
            let planet: Vec<AlertRow> = place
                .fighters
                .iter()
                .filter(|entry| {
                    rebellion_core::squadrons::is_alliance(world, entry.class) == Some(alliance)
                })
                .filter_map(|entry| squadron(entry, false))
                .flatten()
                .collect();
            if !planet.is_empty() {
                rows.push(text(place.name.clone(), AlertFont::Bold));
                rows.extend(planet);
            }
        }
        BattleAlertList::SystemAssets => {
            rows.push(text("System Assets".to_owned(), AlertFont::Heading));
            let facility = |mini: Option<(u32, &'static str)>, object| {
                mini.map(|(mini, label)| {
                    row(
                        label.to_owned(),
                        AlertFont::Plain,
                        object,
                        Some(mini),
                        false,
                        false,
                    )
                })
            };
            rows.extend(place.defense_facilities.iter().filter_map(|&key| {
                let class = world.defense_facilities.get(key)?.class_dat_id;
                facility(defense_facility_mini(class), MiniObject::Craft)
            }));
            rows.extend(place.manufacturing_facilities.iter().filter_map(|&key| {
                let class = world.manufacturing_facilities.get(key)?.class_dat_id;
                facility(manufacturing_facility_mini(class), MiniObject::Craft)
            }));
            rows.extend(place.production_facilities.iter().filter_map(|&key| {
                let class = world.production_facilities.get(key)?.class_dat_id;
                facility(production_facility_mini(class), MiniObject::Craft)
            }));
            rows.extend(
                place
                    .ground_units
                    .iter()
                    .filter_map(|&troop| regiment(troop, false)),
            );
            rows.extend(place.special_forces.iter().filter_map(|&key| {
                let force = world.special_forces.get(key)?;
                let (mini, label) = special_force_mini(force.class_dat_id)?;
                Some(row(
                    label.to_owned(),
                    AlertFont::Plain,
                    MiniObject::SpecialForce(force.class_dat_id),
                    Some(mini),
                    false,
                    false,
                ))
            }));
            rows.extend(
                world
                    .characters
                    .values()
                    .filter(|value| {
                        value.current_system == Some(system) && value.current_fleet.is_none()
                    })
                    .map(|value| character(value, false)),
            );
        }
    }
    rows
}

/// The Battle Alert's list (`FUN_0044f860`, `FUN_00607ea0`): at (83, 40),
/// 256 by 234, rows 40 high, white text. port: no scroll bar; the rows
/// whose middle fits show.
fn battle_alert_list(rows: &[AlertRow]) -> Vec<PageOp> {
    const LEFT: f32 = 83.0;
    const TOP: f32 = 40.0;
    const WIDTH: f32 = 256.0;
    const HEIGHT: f32 = 234.0;
    const ROW: f32 = 40.0;
    let mut ops = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let top = TOP + ROW * index as f32;
        if top + ROW / 2.0 > TOP + HEIGHT {
            break;
        }
        let size = if row.font == AlertFont::Heading {
            18.0
        } else {
            14.0
        };
        let baseline = top + ROW / 2.0 + size / 2.0 - 2.0;
        if row.picture.is_empty() {
            ops.push(PageOp::Text {
                text: row.text.clone(),
                x: LEFT + WIDTH / 2.0,
                baseline,
                size,
                color: WHITE,
                centered: true,
            });
            continue;
        }
        let shift = if row.indent { 20.0 } else { 0.0 };
        for &(source, id) in &row.picture {
            ops.push(PageOp::Bitmap {
                source,
                id,
                x: LEFT + shift,
                y: top,
            });
        }
        ops.push(PageOp::Text {
            text: row.text.clone(),
            x: LEFT + 70.0 + shift,
            baseline,
            size,
            color: WHITE,
            centered: false,
        });
    }
    ops
}

/// The system's side (`+0x24` bits 6..7) and its last arrived side
/// (`+0x78` bits 6..7) as the alert reads them: `Some(true)` for the
/// Alliance, `Some(false)` for the Empire, `None` for neither.
fn battle_alert_sides(world: &GameWorld, system: SystemKey) -> (Option<bool>, Option<bool>) {
    use rebellion_core::dat::Faction;
    let side_of = |faction: Option<Faction>| match faction {
        Some(Faction::Alliance) => Some(true),
        Some(Faction::Empire) => Some(false),
        _ => None,
    };
    let system = world.systems.get(system);
    (
        side_of(system.and_then(|system| system.control.faction())),
        side_of(system.and_then(|system| system.last_arrival)),
    )
}

/// The Battle Alert's title (`+0x54`, `0x7020` "Battle at {1:1}") and
/// summary (`+0x60`), as `FUN_0049c0d0` picks it for the viewer from the
/// system's side and its last arrived side (`+0x78` bits 6..7): on the
/// viewer's system, `0x7022` (the viewer arrived last: breaking the
/// enemy's blockade) or `0x7021` (the enemy threatens it); on a neutral
/// one, `0x7025`; on the enemy's, `0x7024` (the enemy arrived last:
/// breaking the viewer's blockade) or `0x7023` (the viewer's fleet has
/// entered). guide-055 shows `0x7023` for the Empire at Deyer.
fn battle_alert_lines(
    texts: &HashMap<u16, String>,
    viewer_alliance: bool,
    system_name: &str,
    system_side: Option<bool>,
    last_arrival: Option<bool>,
) -> [String; 2] {
    use crate::story_messages::{fill_message_text, side_parameter, MessageParameter};
    let side_word = |alliance: bool| if alliance { "Alliance" } else { "Imperial" };
    let (viewer, enemy) = (side_word(viewer_alliance), side_word(!viewer_alliance));
    let (id, fallback) = match system_side {
        Some(side) if side == viewer_alliance && last_arrival == Some(side) => (
            0x7022,
            format!("The {viewer} fleet is attempting to break the {enemy} blockade of {system_name}."),
        ),
        Some(side) if side == viewer_alliance => (
            0x7021,
            format!("The {enemy} fleet is threatening {system_name}. {viewer} forces are moving to intercept."),
        ),
        None => (
            0x7025,
            format!("{viewer} and {enemy} forces are about to engage in battle near the {system_name} system."),
        ),
        Some(side) if last_arrival == Some(side) => (
            0x7024,
            format!("The {enemy} fleet is attempting to break the {viewer} blockade at {system_name}"),
        ),
        Some(_) => (
            0x7023,
            format!("The {viewer} fleet has entered the {system_name} system. {enemy} forces have been detected on an intercept course."),
        ),
    };
    let viewer = side_parameter(viewer_alliance, texts);
    let enemy = side_parameter(!viewer_alliance, texts);
    let system = MessageParameter::Unit {
        name: system_name,
        class: "",
    };
    let parameters = match id {
        0x7022 | 0x7023 => [viewer, system, enemy],
        0x7021 => [enemy, system, viewer],
        0x7024 => [enemy, viewer, system],
        _ => [viewer, enemy, system],
    };
    let fill = |id: u16, parameters: &[MessageParameter<'_>]| {
        texts.get(&id).map(|template| {
            fill_message_text(template, parameters)
                .trim_end()
                .to_owned()
        })
    };
    [
        fill(0x7020, &[system]).unwrap_or_else(|| format!("Battle at {system_name}")),
        fill(id, &parameters).unwrap_or(fallback),
    ]
}

fn draw_original_battle_alert(
    state: &mut TacticalState,
    cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
) -> TacticalAction {
    let canvas = TacticalCanvas::new(screen_width(), screen_height());
    let session = state.session.as_ref().expect("active Battle Alert session");
    prewarm_tactical_font_atlas(session, canvas, &mut state.warmed_font_sizes);
    let player_alliance = player_is_alliance(session);
    let skin = battle_alert_skin(player_alliance);
    let text_color = if player_alliance {
        Color::from_rgba(255, 45, 40, 255)
    } else {
        Color::from_rgba(40, 255, 65, 255)
    };

    clear_background(BLACK);
    draw_original_bitmap(
        cache,
        DllSource::Strategy,
        if player_alliance {
            resources::strategy::ALLIANCE_COMMAND_CENTER_SHELL
        } else {
            resources::strategy::EMPIRE_COMMAND_CENTER_SHELL
        },
        canvas,
        0.0,
        0.0,
        Some(Rect::new(0.0, 0.0, 640.0, 480.0)),
    );
    draw_strategy_bitmap(
        cache,
        skin.frame,
        canvas,
        BATTLE_ALERT_WINDOW_X,
        BATTLE_ALERT_WINDOW_Y,
    );
    draw_strategy_bitmap(
        cache,
        skin.scene,
        canvas,
        BATTLE_ALERT_WINDOW_X + 12.0,
        BATTLE_ALERT_WINDOW_Y + 13.0,
    );
    draw_battle_alert_text(state, cache, world, troop_transport, canvas, text_color);
    draw_strategy_bitmap(
        cache,
        skin.rail,
        canvas,
        BATTLE_ALERT_WINDOW_X + BATTLE_RESULTS_RAIL_X,
        BATTLE_ALERT_WINDOW_Y,
    );

    let (screen_mouse_x, screen_mouse_y) = mouse_position();
    let mouse = canvas.logical_pointer(screen_mouse_x, screen_mouse_y);
    let local_mouse = (
        mouse.0 - BATTLE_ALERT_WINDOW_X,
        mouse.1 - BATTLE_ALERT_WINDOW_Y,
    );
    let mouse_down = is_mouse_button_down(MouseButton::Left);
    let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);
    let mouse_released = is_mouse_button_released(MouseButton::Left);

    for (index, tab) in BattleAlertTab::ALL.into_iter().enumerate() {
        let rect = skin.tab_rects[index];
        let active = state.battle_alert_tab == tab
            || (mouse_down && rect.contains(local_mouse.0, local_mouse.1));
        draw_strategy_bitmap(
            cache,
            skin.tabs[index][usize::from(active)],
            canvas,
            BATTLE_ALERT_WINDOW_X + rect.x,
            BATTLE_ALERT_WINDOW_Y + rect.y,
        );
        if mouse_released && rect.contains(local_mouse.0, local_mouse.1) {
            state.battle_alert_tab = tab;
            macroquad::logging::info!("[battle_alert] tab={}", tab.label());
        }
    }

    let button_controls = [
        BattleAlertControl::Retreat,
        BattleAlertControl::Simulate,
        BattleAlertControl::TakeCommand,
    ];
    let button_rects = [
        NativeRect::new(12.0, 296.0, 134.0, 27.0),
        NativeRect::new(146.0, 296.0, 134.0, 27.0),
        NativeRect::new(280.0, 296.0, 134.0, 27.0),
    ];
    let retreat_enabled = battle_alert_retreat_enabled(session);
    let hovered = button_rects
        .iter()
        .enumerate()
        .find(|(index, rect)| {
            (*index != 0 || retreat_enabled) && rect.contains(local_mouse.0, local_mouse.1)
        })
        .map(|(index, _)| button_controls[index]);
    if mouse_pressed {
        state.battle_alert_pressed = hovered;
    }

    for (index, rect) in button_rects.into_iter().enumerate() {
        let control = button_controls[index];
        let enabled = control != BattleAlertControl::Retreat || retreat_enabled;
        let resource_state = if !enabled {
            2
        } else if mouse_down
            && state.battle_alert_pressed == Some(control)
            && hovered == Some(control)
        {
            1
        } else {
            0
        };
        draw_strategy_bitmap(
            cache,
            skin.buttons[index][resource_state],
            canvas,
            BATTLE_ALERT_WINDOW_X + rect.x,
            BATTLE_ALERT_WINDOW_Y + rect.y,
        );
    }

    if mouse_released {
        let captured = state.battle_alert_pressed.take();
        if let Some(control) = captured.filter(|control| Some(*control) == hovered) {
            state.battle_alert_open = false;
            return match control {
                BattleAlertControl::Retreat => {
                    macroquad::logging::info!(
                        "[battle_alert] command=retreat resource={} status=simulated",
                        skin.buttons[0][0]
                    );
                    TacticalAction::RetreatBeforeBattle
                }
                BattleAlertControl::Simulate => {
                    macroquad::logging::info!(
                        "[battle_alert] command=simulate resource={} status=routed",
                        skin.buttons[1][0]
                    );
                    TacticalAction::AutoResolve
                }
                BattleAlertControl::TakeCommand => {
                    macroquad::logging::info!(
                        "[battle_alert] command=take_command resource={} status=paused_tactical",
                        skin.buttons[2][0]
                    );
                    TacticalAction::BeginCombat
                }
            };
        }
    } else if !mouse_down {
        state.battle_alert_pressed = None;
    }
    TacticalAction::None
}

fn draw_original_battle_results(
    state: &mut TacticalState,
    cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
) -> TacticalAction {
    let canvas = TacticalCanvas::new(screen_width(), screen_height());
    let session = state.session.as_ref().expect("active result session");
    prewarm_tactical_font_atlas(session, canvas, &mut state.warmed_font_sizes);
    let player_alliance = player_is_alliance(session);
    let skin = battle_result_skin(player_alliance);
    let text_color = if player_alliance {
        Color::from_rgba(255, 45, 40, 255)
    } else {
        Color::from_rgba(40, 255, 65, 255)
    };

    clear_background(BLACK);
    let shell = if player_alliance {
        resources::strategy::ALLIANCE_COMMAND_CENTER_SHELL
    } else {
        resources::strategy::EMPIRE_COMMAND_CENTER_SHELL
    };
    draw_original_bitmap(
        cache,
        DllSource::Strategy,
        shell,
        canvas,
        0.0,
        0.0,
        Some(Rect::new(0.0, 0.0, 640.0, 480.0)),
    );
    draw_strategy_bitmap(
        cache,
        skin.frame,
        canvas,
        BATTLE_RESULTS_WINDOW_X,
        BATTLE_RESULTS_WINDOW_Y,
    );

    let (screen_mouse_x, screen_mouse_y) = mouse_position();
    let mouse = canvas.logical_pointer(screen_mouse_x, screen_mouse_y);
    let mouse_down = is_mouse_button_down(MouseButton::Left);
    let mouse_released = is_mouse_button_released(MouseButton::Left);

    let mut action = match state.battle_result_tab {
        BattleResultTab::Summary => {
            draw_battle_result_summary(cache, canvas, session, world, text_color);
            TacticalAction::None
        }
        BattleResultTab::AllianceForces => {
            draw_battle_result_force_details(
                cache,
                canvas,
                session,
                world,
                troop_transport,
                true,
                state.battle_result_category,
                player_alliance,
                text_color,
            );
            TacticalAction::None
        }
        BattleResultTab::ImperialForces => {
            draw_battle_result_force_details(
                cache,
                canvas,
                session,
                world,
                troop_transport,
                false,
                state.battle_result_category,
                player_alliance,
                text_color,
            );
            TacticalAction::None
        }
        BattleResultTab::GoDirectlyTo => draw_battle_result_destinations(
            cache,
            canvas,
            session,
            player_alliance,
            mouse,
            mouse_down,
            mouse_released,
            text_color,
        ),
    };

    draw_strategy_bitmap(
        cache,
        skin.rail,
        canvas,
        BATTLE_RESULTS_WINDOW_X + BATTLE_RESULTS_RAIL_X,
        BATTLE_RESULTS_WINDOW_Y,
    );
    let close_active = mouse_down
        && skin.close_rect.contains(
            mouse.0 - BATTLE_RESULTS_WINDOW_X,
            mouse.1 - BATTLE_RESULTS_WINDOW_Y,
        );
    draw_strategy_bitmap(
        cache,
        skin.close[usize::from(close_active)],
        canvas,
        BATTLE_RESULTS_WINDOW_X + skin.close_rect.x,
        BATTLE_RESULTS_WINDOW_Y + skin.close_rect.y,
    );
    if mouse_released
        && skin.close_rect.contains(
            mouse.0 - BATTLE_RESULTS_WINDOW_X,
            mouse.1 - BATTLE_RESULTS_WINDOW_Y,
        )
    {
        action = TacticalAction::ReturnToGalaxy;
    }

    for (index, rect) in skin.tab_rects.into_iter().enumerate() {
        let local_mouse = (
            mouse.0 - BATTLE_RESULTS_WINDOW_X,
            mouse.1 - BATTLE_RESULTS_WINDOW_Y,
        );
        let selected = state.battle_result_tab.index() == index;
        let active = selected || (mouse_down && rect.contains(local_mouse.0, local_mouse.1));
        draw_strategy_bitmap(
            cache,
            skin.tabs[index][usize::from(active)],
            canvas,
            BATTLE_RESULTS_WINDOW_X + rect.x,
            BATTLE_RESULTS_WINDOW_Y + rect.y,
        );
        if mouse_released && rect.contains(local_mouse.0, local_mouse.1) {
            state.battle_result_tab = BattleResultTab::from_index(index);
            macroquad::logging::info!(
                "[tactical-results] tab={} category={}",
                state.battle_result_tab.label(),
                state.battle_result_category.label()
            );
        }
    }

    if matches!(
        state.battle_result_tab,
        BattleResultTab::AllianceForces | BattleResultTab::ImperialForces
    ) && mouse_released
    {
        for (index, category) in BattleResultCategory::ALL.into_iter().enumerate() {
            let rect = battle_category_rect(index);
            if rect.contains(
                mouse.0 - BATTLE_RESULTS_WINDOW_X,
                mouse.1 - BATTLE_RESULTS_WINDOW_Y,
            ) {
                state.battle_result_category = category;
                macroquad::logging::info!(
                    "[tactical-results] tab={} category={}",
                    state.battle_result_tab.label(),
                    category.label()
                );
            }
        }
    }
    action
}

/// Draw the tactical combat view (macroquad).
///
/// Call this when `GameMode::TacticalCombat`. Returns a `TacticalAction`
/// indicating whether the main loop should transition modes.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
///
/// # Panics
/// Panics if the active battle session disappears during drawing.
pub fn draw_tactical_view(
    state: &mut TacticalState,
    bmp_cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
) -> TacticalAction {
    if state.session.is_none() {
        return TacticalAction::ReturnToGalaxy;
    }

    if state.battle_alert_open {
        return draw_original_battle_alert(state, bmp_cache, world, troop_transport);
    }

    let mut action = TacticalAction::None;

    // 0. Advance combat if in Combat phase (mutable borrow).
    {
        let session = state.session.as_mut().unwrap();
        if session.phase == BattlePhase::Combat && !session.paused {
            // Step at ~4 ticks per second (at 60fps), scaled by combat_speed.
            let dt = get_frame_time();
            session.advance_presentational_effects(dt);
            print_battle_diagnostics(session);
            let ticks_per_sec = 4.0 * session.combat_speed as f32;
            session.step_accumulator += dt * ticks_per_sec;
            while session.step_accumulator >= 1.0 {
                session.step_accumulator -= 1.0;
                session.step();
                print_battle_diagnostics(session);
                if session.phase == BattlePhase::Results {
                    break;
                }
            }
        }
    }

    if state
        .session
        .as_ref()
        .is_some_and(|session| session.phase == BattlePhase::Results)
    {
        return draw_original_battle_results(state, bmp_cache, world, troop_transport);
    }

    // 1. Paint the unscaled-source tactical shell first, then put the battle
    // only inside its original black aperture. Bitmap 1000 includes every
    // frame edge and the empty right-hand panel housing.
    let canvas = TacticalCanvas::new(screen_width(), screen_height());
    prewarm_tactical_font_atlas(
        state.session.as_ref().unwrap(),
        canvas,
        &mut state.warmed_font_sizes,
    );
    clear_background(BLACK);
    draw_tactical_bitmap(bmp_cache, resources::tactical::BACKGROUND, canvas, 0.0, 0.0);
    set_tactical_aperture_clip(Some(canvas.aperture()));
    let aperture = canvas.aperture();
    let aperture_tuple = (aperture.x, aperture.y, aperture.width, aperture.height);
    update_tactical_camera_follow(state);
    if state.render_original_starfield {
        state.asset_renderer.draw_backdrop(aperture_tuple);
    }
    if state.render_original_planet {
        state.asset_renderer.draw_planet(aperture_tuple);
    }

    // 2. Submit mapped production participants through their original 3D and
    // type-303 resource families. Unmapped or unavailable objects retain the
    // bounded 2D fallback below.
    let (scale, offset_x, offset_y) =
        canvas.arena_transform(state.zoom, state.camera_x, state.camera_y);
    let render_pyrotechnics = state.render_pyrotechnics;
    let (
        production_objects,
        production_fighters,
        production_fields,
        production_projectiles,
        production_effects,
    ) = if state.render_original_participants {
        state
            .session
            .as_ref()
            .map(|session| {
                let ships = session
                    .ships
                    .iter()
                    .enumerate()
                    .filter(|(_, ship)| ship.alive)
                    .filter_map(|(index, ship)| {
                        ship.tactical_resource.map(|resource| TacticalRenderObject {
                            object_id: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                            resource_base: resource.mesh_resource_base,
                            position: ship.source_position.rendered(),
                        })
                    })
                    .collect::<Vec<_>>();
                let fighters = session
                    .fighters
                    .iter()
                    .enumerate()
                    .filter(|(_, fighter)| fighter.alive)
                    .filter_map(|(index, fighter)| {
                        fighter.tactical_resource.map(|resource| {
                            let player_side = fighter.is_attacker == session.player_is_attacker;
                            TacticalFighterRenderObject {
                                object_id: u32::try_from(index)
                                    .unwrap_or(u32::MAX)
                                    .saturating_add(1001),
                                close_resource_id: resource
                                    .grouped_close_resource(player_side, fighter.fighter_group),
                                far_resource_id: resource
                                    .grouped_far_resource(player_side, fighter.fighter_group),
                                indicator_resource_id: resource
                                    .grouped_indicator_resource(player_side, fighter.fighter_group),
                                position: fighter.source_position.rendered(),
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let mut effects = if render_pyrotechnics {
                    session
                        .impact_effects
                        .iter()
                        .filter_map(|effect| {
                            let ship = session.ships.get(effect.target)?;
                            let (source_width, source_height) = effect.sequence.source_size();
                            Some(TacticalEffectRenderObject {
                                object_id: u32::try_from(effect.target)
                                    .unwrap_or(u32::MAX)
                                    .saturating_add(1),
                                resource_id: effect.sequence.resource_base()
                                    + u32::from(effect.frame),
                                source_width,
                                source_height,
                                position: ship.source_position.rendered(),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                if let Some(death_star) = session.death_star {
                    effects.push(TacticalEffectRenderObject {
                        object_id: 3001,
                        resource_id: death_star.resource_id(),
                        source_width: 128,
                        source_height: 128,
                        position: death_star.source_position.rendered(),
                    });
                }
                let fields = session
                    .field_effects
                    .iter()
                    .filter_map(|effect| {
                        let ship = session.ships.get(effect.target)?;
                        Some(TacticalEffectRenderObject {
                            object_id: u32::try_from(effect.target)
                                .unwrap_or(u32::MAX)
                                .saturating_add(1),
                            resource_id: effect.resource_id()?,
                            source_width: 128,
                            source_height: 128,
                            position: ship.source_position.rendered(),
                        })
                    })
                    .collect::<Vec<_>>();
                let mut projectiles = session
                    .weapon_effects
                    .iter()
                    .enumerate()
                    .map(|(index, effect)| TacticalProjectileRenderObject {
                        object_id: u32::try_from(index)
                            .unwrap_or(u32::MAX)
                            .saturating_add(2001),
                        origin: effect.source_position.rendered(),
                        target: effect.target_position.rendered(),
                        progress: effect.progress(),
                        longitudinal_scale: effect.longitudinal_scale,
                        shape_variant: effect.shape_variant,
                        color_selector: effect.color_selector,
                        color: effect.color(),
                    })
                    .collect::<Vec<_>>();
                if let Some(beam) = session.death_star_beam {
                    projectiles.push(TacticalProjectileRenderObject {
                        object_id: 3999,
                        origin: beam.source_position.rendered(),
                        target: beam.target_position.rendered(),
                        progress: (beam.elapsed / beam.duration).clamp(0.0, 1.0),
                        // The source projectile mesh collapses below one device pixel
                        // at the Death Star's tactical scale. Retain a visible authored-
                        // color beam segment while native dimensions remain unproven.
                        longitudinal_scale: 12.0,
                        shape_variant: 3,
                        color_selector: 1,
                        color: Color::new(0.25, 1.0, 0.25, 1.0),
                    });
                }
                (ships, fighters, fields, projectiles, effects)
            })
            .unwrap_or_default()
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };
    let ship_report = state
        .asset_renderer
        .draw_participants(aperture_tuple, &production_objects);
    let fighter_report = state
        .asset_renderer
        .draw_fighters(aperture_tuple, &production_fighters);
    let projectile_report = state
        .asset_renderer
        .draw_projectiles(aperture_tuple, &production_projectiles);
    let _field_report = state
        .asset_renderer
        .draw_fields(aperture_tuple, &production_fields);
    let effect_report = state
        .asset_renderer
        .draw_effects(aperture_tuple, &production_effects);
    if let Some(session) = state.session.as_mut() {
        let mut formation_footprint_changed = false;
        for (object_id, envelope) in &ship_report.collision_envelopes {
            let Some(index) = object_id
                .checked_sub(1)
                .and_then(|index| usize::try_from(index).ok())
            else {
                continue;
            };
            if let Some(ship) = session.ships.get_mut(index) {
                let recovered = OriginalTacticalCollisionEnvelope {
                    vertical_radius: envelope.vertical_radius,
                    planar_diameter: envelope.planar_diameter,
                };
                if ship.source_collision_envelope != Some(recovered) {
                    ship.source_collision_envelope = Some(recovered);
                    formation_footprint_changed = true;
                }
            }
        }
        if formation_footprint_changed && session.combat_tick == 0 && session.paused {
            BattleSession::refresh_original_task_force_geometry(&mut session.ships);
        }
    }
    let rendered_ship_indexes = ship_report
        .rendered_object_ids
        .into_iter()
        .filter_map(|object_id| usize::try_from(object_id.saturating_sub(1)).ok())
        .collect::<Vec<_>>();
    let rendered_fighter_indexes = fighter_report
        .rendered_object_ids
        .into_iter()
        .filter_map(|object_id| usize::try_from(object_id.saturating_sub(1001)).ok())
        .collect::<Vec<_>>();
    let ship_projections = ship_report
        .projections
        .into_iter()
        .filter_map(|projection| {
            usize::try_from(projection.object_id.saturating_sub(1))
                .ok()
                .map(|index| (index, projection))
        })
        .collect::<HashMap<_, _>>();
    let fighter_projections = fighter_report
        .projections
        .into_iter()
        .filter_map(|projection| {
            usize::try_from(projection.object_id.saturating_sub(1001))
                .ok()
                .map(|index| (index, projection))
        })
        .collect::<HashMap<_, _>>();
    let rendered_effect_targets = effect_report
        .rendered_object_ids
        .into_iter()
        .filter_map(|object_id| usize::try_from(object_id.saturating_sub(1)).ok())
        .collect::<HashSet<_>>();
    let rendered_projectiles = projectile_report
        .rendered_object_ids
        .into_iter()
        .filter_map(|object_id| usize::try_from(object_id.saturating_sub(2001)).ok())
        .collect::<HashSet<_>>();
    let navigation_projections = tactical_navigation_projections(state, aperture);
    if state.logged_navigation_scene != Some(state.navigation_sets_visible) {
        let points = navigation_projections
            .iter()
            .map(|projection| {
                format!(
                    "{}:{}:{:.3},{:.3}",
                    projection.set + 1,
                    projection.point + 1,
                    projection.screen.x,
                    projection.screen.y,
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        macroquad::logging::info!(
            "[tactical_navigation] event=scene visible={:?} screen_positions={}",
            state.navigation_sets_visible,
            points,
        );
        state.logged_navigation_scene = Some(state.navigation_sets_visible);
    }

    #[cfg(feature = "interface-test-fixtures")]
    if state.proof_resource_2560 {
        if state.proof_lod_follows_zoom {
            state
                .asset_renderer
                .set_view(TacticalLodView::from_fixture_zoom(state.zoom));
        }
        let aperture = canvas.aperture();
        state
            .asset_renderer
            .draw_proof((aperture.x, aperture.y, aperture.width, aperture.height));
    }

    // Borrow session for rendering (immutable reads).
    let session = state.session.as_ref().unwrap();
    let phase = session.phase;
    let player_is_attacker = session.player_is_attacker;
    let combat_tick = session.combat_tick;
    let paused = session.paused;
    draw_tactical_navigation_points(session, &navigation_projections, canvas.scale);

    // 3. Draw deployment zones (placement phase only).
    if phase == BattlePhase::Placement {
        let zone_w = ARENA_WIDTH * DEPLOY_ZONE_FRACTION * scale;
        draw_rectangle(
            offset_x,
            offset_y,
            zone_w,
            ARENA_HEIGHT * scale,
            Color::new(0.0, 0.15, 0.3, 0.15),
        );
        draw_rectangle_lines(
            offset_x,
            offset_y,
            zone_w,
            ARENA_HEIGHT * scale,
            2.0,
            Color::new(0.0, 0.4, 0.8, 0.5),
        );

        let def_x = offset_x + ARENA_WIDTH * (1.0 - DEPLOY_ZONE_FRACTION) * scale;
        draw_rectangle(
            def_x,
            offset_y,
            zone_w,
            ARENA_HEIGHT * scale,
            Color::new(0.3, 0.05, 0.0, 0.15),
        );
        draw_rectangle_lines(
            def_x,
            offset_y,
            zone_w,
            ARENA_HEIGHT * scale,
            2.0,
            Color::new(0.8, 0.2, 0.0, 0.5),
        );
    }

    // 4. Draw ships (macroquad primitives).
    let aperture = canvas.aperture();
    for (ship_index, ship) in session.ships.iter().enumerate() {
        #[cfg(feature = "interface-test-fixtures")]
        let suppress_fixture_fallback =
            state.suppress_mapped_capital_fallback && ship.tactical_resource.is_some();
        #[cfg(not(feature = "interface-test-fixtures"))]
        let suppress_fixture_fallback = false;
        if !ship.alive || rendered_ship_indexes.contains(&ship_index) || suppress_fixture_fallback {
            continue;
        }
        let sx = offset_x + ship.x * scale;
        let sy = offset_y + ship.y * scale;
        if !aperture.contains(sx, sy) {
            continue;
        }
        let size = DEFAULT_SHIP_SIZE * scale;

        let color = if ship.is_attacker {
            if ship.selected {
                Color::new(0.3, 1.0, 0.3, 0.9)
            } else {
                Color::new(0.2, 0.6, 1.0, 0.8)
            }
        } else {
            Color::new(1.0, 0.3, 0.2, 0.8)
        };

        let half = size / 2.0;
        draw_triangle(
            macroquad::math::Vec2::new(sx, sy - half),
            macroquad::math::Vec2::new(sx - half * 0.6, sy),
            macroquad::math::Vec2::new(sx + half * 0.6, sy),
            color,
        );
        draw_triangle(
            macroquad::math::Vec2::new(sx - half * 0.6, sy),
            macroquad::math::Vec2::new(sx + half * 0.6, sy),
            macroquad::math::Vec2::new(sx, sy + half),
            color,
        );

        let is_alliance = ship.is_attacker == session.attacker_is_alliance;
        let highlighted = if is_alliance {
            state.highlight_alliance
        } else {
            state.highlight_empire
        };
        if highlighted {
            let wire_color = if is_alliance {
                Color::new(0.9, 0.15, 0.12, 0.85)
            } else {
                Color::new(0.1, 0.9, 0.2, 0.85)
            };
            draw_rectangle_lines(
                sx - half * 0.85,
                sy - half * 0.85,
                size * 0.85,
                size * 0.85,
                1.0,
                wire_color,
            );
        }

        let bar_w = size * 0.8;
        let bar_h = 4.0;
        let bar_x = sx - bar_w / 2.0;
        let bar_y = sy + half + 4.0;
        let health_frac = if ship.hull_max > 0 {
            ship.hull_current as f32 / ship.hull_max as f32
        } else {
            0.0
        };
        draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.3, 0.0, 0.0, 0.8));
        let health_color = if health_frac > 0.5 {
            Color::new(0.0, 0.8, 0.0, 0.9)
        } else if health_frac > 0.25 {
            Color::new(0.9, 0.7, 0.0, 0.9)
        } else {
            Color::new(0.9, 0.1, 0.0, 0.9)
        };
        draw_rectangle(bar_x, bar_y, bar_w * health_frac, bar_h, health_color);

        if ship.shield_max > 0 {
            let shield_frac = ship.shield as f32 / ship.shield_max as f32;
            let sbar_y = sy - half - 8.0;
            draw_rectangle(bar_x, sbar_y, bar_w, bar_h, Color::new(0.0, 0.0, 0.3, 0.6));
            draw_rectangle(
                bar_x,
                sbar_y,
                bar_w * shield_frac,
                bar_h,
                Color::new(0.3, 0.5, 1.0, 0.8),
            );
        }

        #[expect(
            clippy::manual_clamp,
            reason = "min/max map NaN to the lower bound; clamp would propagate NaN."
        )]
        let font_size = (12.0 * scale).max(8.0).min(14.0) as u16;
        let label = &ship.name;
        let dims = measure_text(label, None, font_size, 1.0);
        draw_text(
            label,
            sx - dims.width / 2.0,
            bar_y + bar_h + dims.height + 2.0,
            f32::from(font_size),
            WHITE,
        );

        // Focus-fire reticle: draw targeting brackets if this ship is being targeted.
        if ship.attack_target.is_some() && ship.is_attacker == player_is_attacker {
            // Small arrow/marker near ship indicating it has orders.
            draw_circle_lines(sx, sy, half + 4.0, 1.5, Color::new(1.0, 0.6, 0.0, 0.6));
        }

        // Retreat indicator: pulsing "R" above ship.
        if ship.retreating {
            let retreat_alpha = 0.5 + 0.5 * (combat_tick as f32 * 0.3).sin().abs();
            draw_text(
                "RETREAT",
                sx - 20.0,
                sy - half - 14.0,
                10.0,
                Color::new(1.0, 0.5, 0.0, retreat_alpha),
            );
        }
    }

    // The original identifies every protected Escort target with a white box.
    // Use the same retained projections as hit testing so the marker remains
    // attached to the source object at every camera angle and LOD.
    let escort_targets = session
        .ships
        .iter()
        .filter(|ship| ship.alive && ship.order == TacticalOrder::Escort)
        .filter_map(|ship| ship.escort_target)
        .chain(
            session
                .fighters
                .iter()
                .filter(|fighter| fighter.alive && fighter.order == TacticalOrder::Escort)
                .filter_map(|fighter| fighter.escort_target),
        )
        .collect::<HashSet<_>>();
    for target in escort_targets {
        if let Some(projection) = ship_projections.get(&target) {
            draw_projected_frame(*projection, canvas.scale, WHITE);
        }
    }

    // The retained 3D submission and the interaction layer share these exact
    // projected bounds. This replaces the obsolete arena-coordinate frame.
    for (ship_index, projection) in &ship_projections {
        let Some(ship) = session.ships.get(*ship_index) else {
            continue;
        };
        if ship.alive && ship.selected {
            draw_projected_selection_frame(*projection, canvas.scale);
        }
    }

    // 5. Draw fighter squadrons.
    for (fighter_index, fighter) in session.fighters.iter().enumerate() {
        #[cfg(feature = "interface-test-fixtures")]
        let suppress_fixture_fallback =
            state.suppress_mapped_fighter_fallback && fighter.tactical_resource.is_some();
        #[cfg(not(feature = "interface-test-fixtures"))]
        let suppress_fixture_fallback = false;
        if !fighter.alive
            || rendered_fighter_indexes.contains(&fighter_index)
            || suppress_fixture_fallback
        {
            continue;
        }
        let fx = offset_x + fighter.x * scale;
        let fy = offset_y + fighter.y * scale;
        if !aperture.contains(fx, fy) {
            continue;
        }
        let size = FIGHTER_SIZE * scale;

        let color = if fighter.is_attacker {
            Color::new(0.4, 0.8, 1.0, 0.7)
        } else {
            Color::new(1.0, 0.5, 0.3, 0.7)
        };

        let half = size / 2.0;
        draw_triangle(
            macroquad::math::Vec2::new(fx, fy - half),
            macroquad::math::Vec2::new(fx - half, fy + half),
            macroquad::math::Vec2::new(fx + half, fy + half),
            color,
        );

        let label = format!("x{}", fighter.squad_count);
        #[expect(
            clippy::manual_clamp,
            reason = "min/max map NaN to the lower bound; clamp would propagate NaN."
        )]
        let font_size = (10.0 * scale).max(7.0).min(12.0) as u16;
        draw_text(
            &label,
            fx + half + 2.0,
            fy + 4.0,
            f32::from(font_size),
            color,
        );
    }

    for (fighter_index, projection) in &fighter_projections {
        let Some(fighter) = session.fighters.get(*fighter_index) else {
            continue;
        };
        if fighter.alive && fighter.selected {
            draw_projected_selection_frame(*projection, canvas.scale);
        }
    }

    // 6. The normal path above submits the executable's retained projectile
    // meshes. Keep this bounded 2D fallback only when the source camera cannot
    // render a projectile.
    for (effect_index, effect) in session.weapon_effects.iter().enumerate() {
        let source = macroquad::math::Vec2::new(
            offset_x + effect.fallback_source[0] * scale,
            offset_y + effect.fallback_source[1] * scale,
        );
        let target = session.ships.get(effect.target).map_or_else(
            || {
                macroquad::math::Vec2::new(
                    offset_x + effect.fallback_target[0] * scale,
                    offset_y + effect.fallback_target[1] * scale,
                )
            },
            |ship| {
                tactical_ship_screen_position(
                    effect.target,
                    ship,
                    &ship_projections,
                    scale,
                    offset_x,
                    offset_y,
                )
            },
        );

        let alpha = (1.0 - effect.progress()).clamp(0.0, 1.0);
        let base_color = effect.color();
        let color = Color::new(base_color.r, base_color.g, base_color.b, alpha);
        if !rendered_projectiles.contains(&effect_index) {
            let current = source.lerp(target, effect.progress());
            let travel = target - source;
            let direction = if travel.length_squared() > f32::EPSILON {
                travel.normalize()
            } else {
                macroquad::math::Vec2::X
            };
            let tail = current - direction * (2.0 * canvas.scale).max(1.0);
            draw_line(
                tail.x,
                tail.y,
                current.x,
                current.y,
                canvas.scale.max(1.0),
                color,
            );
        }

        // Retain the bounded fallback only when an authentic target frame
        // was unavailable for this event.
        if effect.progress() < 0.375 && !rendered_effect_targets.contains(&effect.target) {
            let flash_r = 4.0 + effect.progress() * 16.0;
            draw_circle(
                target.x,
                target.y,
                flash_r,
                Color::new(1.0, 1.0, 0.8, alpha * 0.6),
            );
        }
    }

    // 6b. Draw targeting lines for both typed source attack targets.
    if phase == BattlePhase::Combat {
        for (ship_index, ship) in session.ships.iter().enumerate() {
            if !ship.alive || !ship.selected || ship.is_attacker != player_is_attacker {
                continue;
            }
            let source = tactical_ship_screen_position(
                ship_index,
                ship,
                &ship_projections,
                scale,
                offset_x,
                offset_y,
            );
            let targets = if ship.manual_targets.is_empty() {
                ship.attack_target.into_iter().collect::<Vec<_>>()
            } else {
                ship.manual_targets.clone()
            };
            for target_spec in targets {
                match target_spec {
                    TacticalAttackTarget::CapitalShip(target_index) => {
                        let Some(target_ship) = session.ships.get(target_index) else {
                            continue;
                        };
                        if !target_ship.alive {
                            continue;
                        }
                        let target = tactical_ship_screen_position(
                            target_index,
                            target_ship,
                            &ship_projections,
                            scale,
                            offset_x,
                            offset_y,
                        );
                        let radius = ship_projections.get(&target_index).map_or(
                            DEFAULT_SHIP_SIZE * scale * 0.4,
                            |projection| {
                                (projection.max - projection.min).max_element() * 0.6 + 3.0
                            },
                        );
                        draw_tactical_attack_target(source, target, radius);
                    }
                    TacticalAttackTarget::FighterGroup(target_index) => {
                        let Some(target_fighter) = session.fighters.get(target_index) else {
                            continue;
                        };
                        if !target_fighter.alive {
                            continue;
                        }
                        let target = tactical_fighter_screen_position(
                            target_index,
                            target_fighter,
                            &fighter_projections,
                            scale,
                            offset_x,
                            offset_y,
                        );
                        let radius = fighter_projections.get(&target_index).map_or(
                            FIGHTER_SIZE * scale * 0.75,
                            |projection| {
                                (projection.max - projection.min).max_element() * 0.6 + 3.0
                            },
                        );
                        draw_tactical_attack_target(source, target, radius);
                    }
                }
            }
        }

        for (fighter_index, fighter) in session.fighters.iter().enumerate() {
            if !fighter.alive || !fighter.selected || fighter.is_attacker != player_is_attacker {
                continue;
            }
            let source = tactical_fighter_screen_position(
                fighter_index,
                fighter,
                &fighter_projections,
                scale,
                offset_x,
                offset_y,
            );
            match fighter.attack_target {
                Some(TacticalAttackTarget::CapitalShip(target_index)) => {
                    let Some(target_ship) = session.ships.get(target_index) else {
                        continue;
                    };
                    if !target_ship.alive {
                        continue;
                    }
                    let target = tactical_ship_screen_position(
                        target_index,
                        target_ship,
                        &ship_projections,
                        scale,
                        offset_x,
                        offset_y,
                    );
                    let radius = ship_projections
                        .get(&target_index)
                        .map_or(DEFAULT_SHIP_SIZE * scale * 0.4, |projection| {
                            (projection.max - projection.min).max_element() * 0.6 + 3.0
                        });
                    draw_tactical_attack_target(source, target, radius);
                }
                Some(TacticalAttackTarget::FighterGroup(target_index)) => {
                    let Some(target_fighter) = session.fighters.get(target_index) else {
                        continue;
                    };
                    if !target_fighter.alive {
                        continue;
                    }
                    let target = tactical_fighter_screen_position(
                        target_index,
                        target_fighter,
                        &fighter_projections,
                        scale,
                        offset_x,
                        offset_y,
                    );
                    let radius = fighter_projections
                        .get(&target_index)
                        .map_or(FIGHTER_SIZE * scale * 0.75, |projection| {
                            (projection.max - projection.min).max_element() * 0.6 + 3.0
                        });
                    draw_tactical_attack_target(source, target, radius);
                }
                None => {}
            }
        }
    }

    // End immutable borrow before mutable operations.
    let _ = session;

    // Restore full-canvas painting before HUD controls and any later egui pass.
    set_tactical_aperture_clip(None);

    // 6. Handle input per phase.
    if phase == BattlePhase::Placement {
        let session = state.session.as_mut().unwrap();
        handle_placement_input(
            session,
            &mut state.dragging_ship,
            &mut state.drag_offset,
            scale,
            offset_x,
            offset_y,
        );
    } else if phase == BattlePhase::Combat {
        let session = state.session.as_mut().unwrap();
        handle_combat_input(
            session,
            &mut state.death_star_targeting,
            scale,
            offset_x,
            offset_y,
            canvas.aperture(),
            &ship_projections,
            &navigation_projections,
        );
    }

    if phase == BattlePhase::Combat {
        action = handle_original_tactical_controls(state, bmp_cache, canvas);
        let pressed_control = pressed_tactical_hud_control(bmp_cache, canvas);
        let player_is_alliance = state
            .session
            .as_ref()
            .is_some_and(|session| session.player_is_attacker == session.attacker_is_alliance);
        let groups = TacticalGroupHud::from_session(state.session.as_ref().unwrap());
        let death_star = state.session.as_ref().and_then(player_death_star);
        let has_selection =
            groups.selected_fighter_group.is_some() || groups.selected_ship_name.is_some();
        let attack_death_star_enabled = state
            .session
            .as_ref()
            .is_some_and(attack_death_star_enabled);
        let withdraw_enabled = state
            .session
            .as_ref()
            .is_some_and(tactical_withdraw_enabled);
        let pressed_options = state
            .battle_options_open
            .then(|| pressed_tactical_battle_options_control(bmp_cache, canvas, withdraw_enabled))
            .flatten()
            .filter(|control| state.battle_options_pressed == Some(*control));
        let pressed_withdraw_confirmation = state
            .withdraw_confirmation_open
            .then(|| pressed_tactical_withdraw_confirmation_control(bmp_cache, canvas));
        let pressed_command = (!state.battle_options_open
            && !state.withdraw_confirmation_open
            && !state.player_observing)
            .then(|| {
                pressed_tactical_command_control(
                    bmp_cache,
                    canvas,
                    state.command_panel,
                    has_selection,
                    groups.selected_fighter_group.is_some(),
                    attack_death_star_enabled,
                    player_is_alliance,
                )
            });
        draw_original_tactical_hud(
            bmp_cache,
            canvas,
            player_is_alliance,
            groups,
            attack_death_star_enabled,
            paused,
            state.highlight_alliance,
            state.highlight_empire,
            state.navigation_sets_visible,
            pressed_control,
            state.command_panel,
            pressed_command.flatten(),
            state.battle_options_open,
            state.withdraw_confirmation_open,
            state.player_observing,
            withdraw_enabled,
            pressed_options,
            pressed_withdraw_confirmation
                .flatten()
                .filter(|control| state.withdraw_confirmation_pressed == Some(*control)),
            death_star,
            state.death_star_laser_pressed,
            state.death_star_targeting,
        );
        if paused {
            let (x, y) = canvas.point(25.0, 48.0);
            draw_text("Battle Paused.", x, y, 16.0 * canvas.scale, RED);
        }
    }

    action
}

// ---------------------------------------------------------------------------
// Combat input handling
// ---------------------------------------------------------------------------

fn tactical_ship_screen_position(
    ship_index: usize,
    ship: &TacticalShip,
    projections: &HashMap<usize, TacticalScreenProjection>,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
) -> macroquad::math::Vec2 {
    projections.get(&ship_index).map_or_else(
        || macroquad::math::Vec2::new(offset_x + ship.x * scale, offset_y + ship.y * scale),
        |projection| projection.center,
    )
}

fn tactical_fighter_screen_position(
    fighter_index: usize,
    fighter: &TacticalFighter,
    projections: &HashMap<usize, TacticalScreenProjection>,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
) -> macroquad::math::Vec2 {
    projections.get(&fighter_index).map_or_else(
        || macroquad::math::Vec2::new(offset_x + fighter.x * scale, offset_y + fighter.y * scale),
        |projection| projection.center,
    )
}

#[derive(Debug, Clone, Copy)]
struct TacticalNavigationProjection {
    set: u8,
    point: u8,
    position: TacticalWorldPosition,
    screen: macroquad::math::Vec2,
}

fn tactical_navigation_projections(
    state: &TacticalState,
    aperture: NativeRect,
) -> Vec<TacticalNavigationProjection> {
    let Some(session) = state.session.as_ref() else {
        return Vec::new();
    };
    let aperture = (aperture.x, aperture.y, aperture.width, aperture.height);
    session
        .navigation_sets
        .iter()
        .enumerate()
        .filter(|(set, _)| state.navigation_sets_visible[*set])
        .flat_map(|(set, points)| {
            points
                .iter()
                .enumerate()
                .filter_map(move |(point, position)| {
                    state
                        .asset_renderer
                        .project_source_position(aperture, position.rendered())
                        .map(|screen| TacticalNavigationProjection {
                            set: u8::try_from(set).unwrap_or(3),
                            point: u8::try_from(point).unwrap_or(u8::MAX),
                            position: *position,
                            screen,
                        })
                })
        })
        .collect()
}

fn draw_tactical_navigation_points(
    session: &BattleSession,
    projections: &[TacticalNavigationProjection],
    canvas_scale: f32,
) {
    let colors = [RED, GREEN, BLUE, YELLOW];
    let selected_routes = session
        .ships
        .iter()
        .filter(|ship| ship.selected && ship.is_attacker == session.player_is_attacker)
        .flat_map(|ship| ship.navigation_route.iter().copied())
        .collect::<Vec<_>>();
    for projection in projections {
        let assigned = selected_routes.contains(&projection.position);
        let color = colors[usize::from(projection.set.min(3))];
        let radius = if assigned { 7.0 } else { 5.0 } * canvas_scale;
        let top = vec2(projection.screen.x, projection.screen.y - radius);
        let left = vec2(
            projection.screen.x - radius,
            projection.screen.y + radius * 0.75,
        );
        let right = vec2(
            projection.screen.x + radius,
            projection.screen.y + radius * 0.75,
        );
        draw_triangle(top, left, right, color);
        draw_triangle_lines(top, left, right, canvas_scale.max(1.0), WHITE);
        if assigned {
            draw_circle_lines(
                projection.screen.x,
                projection.screen.y,
                radius + 2.0 * canvas_scale,
                canvas_scale.max(1.0),
                WHITE,
            );
        }
    }
}

fn draw_tactical_attack_target(
    source: macroquad::math::Vec2,
    target: macroquad::math::Vec2,
    radius: f32,
) {
    draw_line(
        source.x,
        source.y,
        target.x,
        target.y,
        1.0,
        Color::new(1.0, 0.5, 0.0, 0.4),
    );
    draw_circle_lines(
        target.x,
        target.y,
        radius,
        1.5,
        Color::new(1.0, 0.3, 0.0, 0.7),
    );
}

fn draw_projected_selection_frame(projection: TacticalScreenProjection, canvas_scale: f32) {
    draw_projected_frame(projection, canvas_scale, Color::new(1.0, 0.82, 0.12, 0.95));
}

fn draw_projected_frame(projection: TacticalScreenProjection, canvas_scale: f32, color: Color) {
    let padding = 3.0 * canvas_scale;
    let minimum_size = 10.0 * canvas_scale;
    let minimum_half = macroquad::math::Vec2::splat(minimum_size * 0.5);
    let origin = (projection.min - macroquad::math::Vec2::splat(padding))
        .min(projection.center - minimum_half);
    let max = (projection.max + macroquad::math::Vec2::splat(padding))
        .max(projection.center + minimum_half);
    let size = max - origin;
    let corner = (size.min_element() * 0.28).clamp(3.0 * canvas_scale, 8.0 * canvas_scale);
    let thickness = canvas_scale.max(1.0);
    for (x, x_direction) in [(origin.x, 1.0), (origin.x + size.x, -1.0)] {
        for (y, y_direction) in [(origin.y, 1.0), (origin.y + size.y, -1.0)] {
            draw_line(x, y, x + corner * x_direction, y, thickness, color);
            draw_line(x, y, x, y + corner * y_direction, thickness, color);
        }
    }
}

/// Handle mouse input during the combat phase.
///
/// - Left-click: select a player ship.
/// - Right-click on friendly: issue the original order-1 Escort command.
/// - Right-click on enemy: issue focus-fire order to all selected player ships.
/// - R key: retreat selected ships.
fn handle_combat_input(
    session: &mut BattleSession,
    death_star_targeting: &mut bool,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    aperture: NativeRect,
    projections: &HashMap<usize, TacticalScreenProjection>,
    navigation_projections: &[TacticalNavigationProjection],
) {
    let (mx, my) = mouse_position();
    let inside_battle = aperture.contains(mx, my);
    let arena_pointer_x = (mx - offset_x) / scale;
    let arena_pointer_y = (my - offset_y) / scale;
    let hit_radius = DEFAULT_SHIP_SIZE * 0.6;
    let screen_pointer = macroquad::math::Vec2::new(mx, my);
    let projected_hit_radius = 7.0 * (aperture.width / BATTLE_APERTURE.width);
    let navigation_hit = navigation_projections.iter().find(|projection| {
        projection.screen.distance(screen_pointer) <= 8.0 * (aperture.width / BATTLE_APERTURE.width)
    });
    let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);

    if inside_battle && is_mouse_button_pressed(MouseButton::Left) && control {
        if let Some(navigation) = navigation_hit {
            let mut changed = 0;
            for ship in &mut session.ships {
                if !ship.selected || !ship.alive || ship.is_attacker != session.player_is_attacker {
                    continue;
                }
                let before = ship.navigation_route.len();
                ship.navigation_route
                    .retain(|point| *point != navigation.position);
                if before != ship.navigation_route.len() {
                    changed += 1;
                    ship.source_waypoint = ship.navigation_route.first().copied();
                    ship.source_desired_forward = ship
                        .source_waypoint
                        .map_or(TacticalWorldVector::ZERO, |waypoint| {
                            source_delta(waypoint, ship.source_position).normalized()
                        });
                }
            }
            macroquad::logging::info!(
                "[tactical_navigation] event=route_remove set={} point={} members={}",
                navigation.set + 1,
                navigation.point + 1,
                changed,
            );
            return;
        }
    }

    // Left-click: select player's ship.
    if inside_battle && is_mouse_button_pressed(MouseButton::Left) {
        let mut hit = None;
        for (i, ship) in session.ships.iter().enumerate() {
            if !ship.alive || ship.retreating {
                continue;
            }
            if ship.is_attacker != session.player_is_attacker {
                continue;
            }
            let projected_hit = projections.get(&i).is_some_and(|projection| {
                projection.contains(screen_pointer, projected_hit_radius)
            });
            let dx = arena_pointer_x - ship.x;
            let dy = arena_pointer_y - ship.y;
            if projected_hit
                || (!projections.contains_key(&i) && dx * dx + dy * dy < hit_radius * hit_radius)
            {
                hit = Some(i);
                break;
            }
        }

        if let Some(idx) = hit {
            // The original manual uses Ctrl to build a multi-ship selection
            // before assigning those ships to a task force.
            if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
                session.ships[idx].selected = !session.ships[idx].selected;
            } else {
                for s in &mut session.ships {
                    s.selected = false;
                }
                session.ships[idx].selected = true;
            }
            session.selected_ship = Some(idx);
            session.selected_fighter_group = None;
            for fighter in &mut session.fighters {
                fighter.selected = false;
            }
            macroquad::logging::info!(
                "[tactical_3d] selection object_id={} source_projection={}",
                idx + 1,
                projections.contains_key(&idx)
            );
        } else {
            // Clicked empty space — deselect all.
            for s in &mut session.ships {
                s.selected = false;
            }
            session.selected_ship = None;
            session.selected_fighter_group = None;
            for fighter in &mut session.fighters {
                fighter.selected = false;
            }
        }
    }

    // Right-click: assign an armed Death Star superlaser or issue focus fire.
    if inside_battle && is_mouse_button_pressed(MouseButton::Right) {
        if let Some(navigation) = navigation_hit {
            let mut assigned = 0;
            let mut longest_route = 0;
            for ship in &mut session.ships {
                if !ship.selected || !ship.alive || ship.is_attacker != session.player_is_attacker {
                    continue;
                }
                ship.order = TacticalOrder::None;
                ship.escort_target = None;
                if control {
                    if !ship.navigation_route.contains(&navigation.position) {
                        ship.navigation_route.push(navigation.position);
                    }
                } else {
                    ship.navigation_route.clear();
                    ship.navigation_route.push(navigation.position);
                }
                ship.source_waypoint = ship.navigation_route.first().copied();
                ship.source_desired_forward = ship
                    .source_waypoint
                    .map_or(TacticalWorldVector::ZERO, |waypoint| {
                        source_delta(waypoint, ship.source_position).normalized()
                    });
                assigned += 1;
                longest_route = longest_route.max(ship.navigation_route.len());
            }
            macroquad::logging::info!(
                "[tactical_navigation] event=route_assign set={} point={} append={} members={} route_len={}",
                navigation.set + 1,
                navigation.point + 1,
                control,
                assigned,
                longest_route,
            );
            return;
        }
        let mut target_hit = None;
        for (i, ship) in session.ships.iter().enumerate() {
            if !ship.alive {
                continue;
            }
            // Can only target enemy ships.
            if ship.is_attacker == session.player_is_attacker {
                continue;
            }
            let projected_hit = projections.get(&i).is_some_and(|projection| {
                projection.contains(screen_pointer, projected_hit_radius)
            });
            let dx = arena_pointer_x - ship.x;
            let dy = arena_pointer_y - ship.y;
            if projected_hit
                || (!projections.contains_key(&i) && dx * dx + dy * dy < hit_radius * hit_radius)
            {
                target_hit = Some(i);
                break;
            }
        }

        if *death_star_targeting {
            if let Some(target_idx) = target_hit {
                let committed = session.commit_death_star_shot(target_idx);
                print_battle_diagnostics(session);
                if committed {
                    *death_star_targeting = false;
                }
            }
            return;
        }

        let friendly_hit = session.ships.iter().enumerate().find_map(|(index, ship)| {
            if !ship.alive || ship.retreating || ship.is_attacker != session.player_is_attacker {
                return None;
            }
            let projected_hit = projections.get(&index).is_some_and(|projection| {
                projection.contains(screen_pointer, projected_hit_radius)
            });
            let dx = arena_pointer_x - ship.x;
            let dy = arena_pointer_y - ship.y;
            (projected_hit
                || (!projections.contains_key(&index)
                    && dx * dx + dy * dy < hit_radius * hit_radius))
                .then_some(index)
        });
        if let Some(target_idx) = friendly_hit {
            let (capital_members, fighter_members) = assign_selected_escort(session, target_idx);
            if capital_members + fighter_members > 0 {
                macroquad::logging::info!(
                    "[tactical_escort] event=assigned order=1 target_object_id={} capital_members={} fighter_members={} source_projection={}",
                    target_idx + 1,
                    capital_members,
                    fighter_members,
                    projections.contains_key(&target_idx),
                );
            }
            return;
        }

        if let Some(target_idx) = target_hit {
            // Assign one or several ordered focus targets to selected ships.
            match assign_selected_capital_focus_target(session, target_idx, control) {
                TacticalFocusAssignment::Assigned {
                    object_ids,
                    maximum_targets,
                } => {
                    macroquad::logging::info!(
                        "[tactical_3d] focus source_object_ids={} target_object_id={} source_projection={}",
                        object_ids
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                        target_idx + 1,
                        projections.contains_key(&target_idx)
                            && object_ids
                                .iter()
                                .all(|object_id| projections.contains_key(&(*object_id - 1)))
                    );
                    macroquad::logging::info!(
                        "[tactical_navigation] event=target_assign status=assigned append={} target_object_id={} members={} target_count={}",
                        control,
                        target_idx + 1,
                        object_ids.len(),
                        maximum_targets,
                    );
                }
                TacticalFocusAssignment::RejectedMixedTaskForces { member_count } => {
                    macroquad::logging::info!(
                        "[tactical_navigation] event=target_assign status=rejected_mixed_task_forces append={} target_object_id={} members={}",
                        control,
                        target_idx + 1,
                        member_count,
                    );
                }
            }
        } else {
            // Right-clicked empty space — clear focus targets for selected ships.
            for ship in &mut session.ships {
                if ship.selected && ship.is_attacker == session.player_is_attacker {
                    ship.attack_target = None;
                    ship.manual_targets.clear();
                    ship.escort_target = None;
                    if ship.order == TacticalOrder::Escort {
                        ship.order = TacticalOrder::None;
                    }
                }
            }
        }
    }

    // R key: retreat selected ships.
    if is_key_pressed(KeyCode::R) {
        for ship in &mut session.ships {
            if ship.selected && ship.is_attacker == session.player_is_attacker && ship.alive {
                ship.retreating = true;
                ship.selected = false;
            }
        }
        session.selected_ship = None;
        session.selected_fighter_group = None;
    }
}

// ---------------------------------------------------------------------------
// Placement input handling
// ---------------------------------------------------------------------------

fn handle_placement_input(
    session: &mut BattleSession,
    dragging_ship: &mut Option<usize>,
    drag_offset: &mut (f32, f32),
    scale: f32,
    offset_x: f32,
    offset_y: f32,
) {
    let (mx, my) = mouse_position();

    // Convert mouse to arena coordinates.
    let arena_pointer_x = (mx - offset_x) / scale;
    let arena_pointer_y = (my - offset_y) / scale;

    // Deployment zone bounds for the player's side.
    let (zone_min_x, zone_max_x) = if session.player_is_attacker {
        (0.0, ARENA_WIDTH * DEPLOY_ZONE_FRACTION)
    } else {
        (ARENA_WIDTH * (1.0 - DEPLOY_ZONE_FRACTION), ARENA_WIDTH)
    };

    if is_mouse_button_pressed(MouseButton::Left) {
        // Check if clicking on a player's ship.
        let mut hit = None;
        for (i, ship) in session.ships.iter().enumerate() {
            if !ship.alive {
                continue;
            }
            if ship.is_attacker != session.player_is_attacker {
                continue;
            }
            let dx = arena_pointer_x - ship.x;
            let dy = arena_pointer_y - ship.y;
            if dx * dx + dy * dy < (DEFAULT_SHIP_SIZE * 0.6).powi(2) {
                hit = Some(i);
                break;
            }
        }

        if let Some(idx) = hit {
            *dragging_ship = Some(idx);
            *drag_offset = (
                session.ships[idx].x - arena_pointer_x,
                session.ships[idx].y - arena_pointer_y,
            );
            // Select this ship.
            for s in &mut session.ships {
                s.selected = false;
            }
            session.ships[idx].selected = true;
            session.selected_ship = Some(idx);
        } else {
            // Deselect.
            for s in &mut session.ships {
                s.selected = false;
            }
            session.selected_ship = None;
        }
    }

    if is_mouse_button_down(MouseButton::Left) {
        if let Some(idx) = *dragging_ship {
            let new_x = (arena_pointer_x + drag_offset.0).clamp(
                zone_min_x + DEFAULT_SHIP_SIZE * 0.5,
                zone_max_x - DEFAULT_SHIP_SIZE * 0.5,
            );
            let new_y = (arena_pointer_y + drag_offset.1).clamp(
                DEFAULT_SHIP_SIZE * 0.5,
                ARENA_HEIGHT - DEFAULT_SHIP_SIZE * 0.5,
            );
            session.ships[idx].x = new_x;
            session.ships[idx].y = new_y;
        }
    }

    if is_mouse_button_released(MouseButton::Left) {
        *dragging_ship = None;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey};
    use rebellion_core::tactical_resources::{
        capital_ship_tactical_resource, death_star_tactical_resource, fighter_tactical_resource,
        DEATH_STAR_TACTICAL_RESOURCE,
    };

    fn test_ship(dat_id: u32, roster: usize, is_alliance: bool, alive: bool) -> TacticalShip {
        TacticalShip {
            class_key: CapitalShipKey::default(),
            identity: TacticalObjectIdentity {
                class_dat_id: DatId::new(dat_id),
                fleet_roster_index: roster,
                is_alliance,
            },
            tactical_resource: capital_ship_tactical_resource(DatId::new(dat_id)),
            death_star_resource: death_star_tactical_resource(DatId::new(dat_id)),
            source_position: TacticalWorldPosition::ORIGIN,
            source_forward: if is_alliance {
                TacticalWorldVector::SOURCE_FORWARD
            } else {
                TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                }
            },
            source_desired_forward: if is_alliance {
                TacticalWorldVector::SOURCE_FORWARD
            } else {
                TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                }
            },
            source_velocity: TacticalWorldVector::ZERO,
            source_waypoint: None,
            navigation_route: Vec::new(),
            source_collision_envelope: None,
            name: format!("ship-{dat_id}"),
            x: 0.0,
            y: 0.0,
            hull_current: 1,
            hull_max: 1,
            shield: 0,
            shield_max: 0,
            subsystem_condition: TacticalSubsystemCondition {
                shields: 0,
                weapons: 0,
                tractor: 0,
                engines: 0,
                hyperdrive: 0,
            },
            subsystem_capacity: TacticalSubsystemCapacity {
                shields: false,
                weapons: false,
                tractor: false,
                engines: false,
                hyperdrive: 0,
            },
            subsystem_damage: TacticalSubsystemDamage::default(),
            sublight_engine_power: 100.0,
            maneuverability: 0,
            tractor_beam_power: 25.0,
            fighter_capacity: 1,
            maneuver_state_value: None,
            damage_control: 100,
            is_attacker: is_alliance,
            alive,
            selected: false,
            order: TacticalOrder::None,
            tactic: TacticalTactic::StandOff,
            task_force: u8::try_from(roster.min(7)).unwrap_or(7),
            fleet_ship_index: roster,
            sprite_id: None,
            weapon_arcs: [TacticalWeaponArc::default(); 4],
            weapon_ranges: TacticalWeaponRanges::default(),
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

    fn test_fighter(dat_id: u32, is_alliance: bool) -> TacticalFighter {
        TacticalFighter {
            class_key: FighterKey::default(),
            identity: TacticalObjectIdentity {
                class_dat_id: DatId::new(dat_id),
                fleet_roster_index: 0,
                is_alliance,
            },
            tactical_resource: fighter_tactical_resource(DatId::new(dat_id)),
            source_position: TacticalWorldPosition::ORIGIN,
            sublight_engine_power: 10.0,
            fleet_fighter_index: 0,
            fleet_squadron_index: 0,
            name: format!("fighter-{dat_id}"),
            x: 0.0,
            y: 0.0,
            squad_count: 12,
            hull_current: 96.0,
            hull_max: 96.0,
            shield: 5.0,
            shield_max: 5.0,
            weapon_arc: TacticalWeaponArc::new(5, 0, 0),
            weapon_ranges: TacticalWeaponRanges {
                laser_cannon: 12.0,
                ion_cannon: 0.0,
                turbolaser: 0.0,
            },
            torpedo_strength: 0,
            torpedo_range: 0.0,
            maneuverability: 5,
            is_attacker: is_alliance,
            alive: true,
            fighter_group: 0,
            selected: false,
            order: TacticalOrder::None,
            tactic: TacticalTactic::StandOff,
            attack_target: None,
            escort_target: None,
            recovery_state: TacticalFighterRecoveryState::AwaitingCarrier,
            recovery_target: None,
        }
    }

    fn test_session(
        ships: Vec<TacticalShip>,
        fighters: Vec<TacticalFighter>,
        player_is_attacker: bool,
    ) -> BattleSession {
        BattleSession {
            system: SystemKey::default(),
            system_name: "Test System".to_string(),
            system_picture_id: 1,
            attacker_fleet: FleetKey::default(),
            defender_fleet: FleetKey::default(),
            attacker_is_alliance: true,
            player_is_attacker,
            phase: BattlePhase::Combat,
            ships,
            fighters,
            death_star: None,
            death_star_beam: None,
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating: 1,
            trench_run_pending_outcome: None,
            trench_run_participants: Vec::new(),
            trench_run_chatter: [ORIGINAL_TRENCH_RUN_SILENT_EVENT; 9],
            trench_run_chatter_cursor: 0,
            trench_run_cinematic_pending: false,
            navigation_sets: original_navigation_point_sets(OriginalTacticalLayout::default()),
            source_layout: OriginalTacticalLayout::default(),
            selected_ship: None,
            selected_fighter_group: None,
            placement_confirmed: true,
            start_tick: 0,
            combat_tick: 0,
            tactical_rng: OriginalTacticalRng::new(0x5eed_ba77),
            weapon_effects: Vec::new(),
            impact_effects: Vec::new(),
            pending_audio_cues: Vec::new(),
            pending_voice_cues: Vec::new(),
            field_effects: Vec::new(),
            subsystem_repairs: Vec::new(),
            paused: true,
            combat_speed: 1,
            step_accumulator: 0.0,
            winner: None,
            diagnostic_log: Vec::new(),
        }
    }

    #[test]
    fn the_battle_summary_columns_hold_operational_and_destroyed_units() {
        // FUN_004714e0: a damaged ship stays operational; only the lost
        // go to Destroyed. Personnel take three columns.
        let mut damaged = test_ship(64, 1, true, true);
        damaged.hull_current = 40;
        damaged.hull_max = 100;
        let mut lost = test_ship(64, 2, true, true);
        lost.alive = false;
        let session = test_session(vec![damaged, lost], Vec::new(), true);
        let world = GameWorld::default();
        let transport = TroopTransportState::default();
        let entries = battle_result_entries(
            &session,
            &world,
            &transport,
            true,
            BattleResultCategory::CapitalShips,
        );
        let columns = battle_result_columns(&entries, false);
        assert_eq!((columns[0].len(), columns[1].len()), (1, 1));
        assert_eq!(battle_result_columns(&entries, true).len(), 3);
        assert_eq!(battle_result_columns(&entries, true)[1].len(), 0);
        // Empty columns: the first "No Survivors" unless all are empty,
        // the middle "None", the last "No Casualties".
        assert_eq!(battle_result_empty_labels(&columns), [None, None]);
        let only_lost = vec![Vec::new(), columns[1].clone()];
        assert_eq!(
            battle_result_empty_labels(&only_lost),
            [Some("No Survivors"), None]
        );
        let none: Vec<Vec<&BattleResultEntry>> = vec![Vec::new(), Vec::new(), Vec::new()];
        assert_eq!(
            battle_result_empty_labels(&none),
            [Some("None"), Some("None"), Some("No Casualties")]
        );
    }

    #[test]
    fn a_unit_is_destroyed_only_when_lost() {
        // FUN_004714e0: Destroyed holds the units the battle ended; every
        // other stays Operational, damaged or not.
        let mut whole = test_ship(64, 1, true, true);
        whole.hull_current = 100;
        let mut sunk = test_ship(64, 2, true, true);
        sunk.hull_current = 0;
        let lost = test_ship(64, 3, true, false);
        let enemy = test_ship(64, 4, false, true);
        let mut fighters = vec![test_fighter(1, true), test_fighter(1, true), test_fighter(1, true)];
        fighters[1].squad_count = 0;
        fighters[2].alive = false;
        let session = test_session(vec![whole, sunk, lost, enemy], fighters, true);
        let world = GameWorld::default();
        let transport = TroopTransportState::default();
        let conditions = |category| {
            battle_result_entries(&session, &world, &transport, true, category)
                .into_iter()
                .map(|entry| entry.condition)
                .collect::<Vec<_>>()
        };
        use BattleResultCondition::{Destroyed, Operational};
        assert_eq!(
            conditions(BattleResultCategory::CapitalShips),
            [Operational, Destroyed, Destroyed]
        );
        assert_eq!(
            conditions(BattleResultCategory::Fighters),
            [Operational, Destroyed, Destroyed]
        );
        // A Death Star is lost when destroyed or out of hull.
        let mut session = session;
        let mut death_star = |destroyed, hull| {
            session.death_star = Some(rebellion_core::tactical::TacticalDeathStar {
                resource: rebellion_core::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
                is_attacker: true,
                is_alliance: true,
                source_position: TacticalWorldPosition::ORIGIN,
                hull,
                laser_charge: 0.0,
                shielded: false,
                destroyed,
                action_committed: false,
            });
            battle_result_entries(&session, &world, &transport, true, BattleResultCategory::CapitalShips)
                .last()
                .map(|entry| (entry.name.clone(), entry.condition))
                .unwrap()
        };
        assert_eq!(death_star(false, 5.0), ("Death Star".into(), Operational));
        assert_eq!(death_star(true, 5.0).1, Destroyed);
        assert_eq!(death_star(false, 0.0).1, Destroyed);
    }

    #[test]
    fn a_ships_picture_is_its_classs_portrait_and_two_rows_fit() {
        // FUN_0042c3b0(.., 1, 1): the mini less 0x4000; the list's 166
        // fits two 70-high rows (FUN_004714e0's 165 by 70 cells).
        let mut world = GameWorld::default();
        let class = world.capital_ship_classes.insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            ..Default::default()
        });
        let ships: Vec<_> = (1..=3)
            .map(|roster| {
                let mut ship = test_ship(64, roster, true, true);
                ship.class_key = class;
                ship
            })
            .collect();
        let session = test_session(ships, Vec::new(), true);
        let transport = TroopTransportState::default();
        let entries = battle_result_entries(
            &session,
            &world,
            &transport,
            true,
            BattleResultCategory::CapitalShips,
        );
        let mini = crate::panels::fleets::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap();
        assert_eq!(entries[0].picture, Some((DllSource::Gokres, mini - 0x4000)));
        assert_eq!(entries[0].backdrop, None);
        let ops = battle_force_page(
            "Hoth",
            &entries,
            true,
            BattleResultCategory::CapitalShips,
            true,
            WHITE,
            |_, _| Some([10, 10]),
        );
        let pictures = ops
            .iter()
            .filter(|op| matches!(op, PageOp::Bitmap { id, .. } if *id == mini - 0x4000))
            .count();
        assert_eq!(pictures, 2);
    }

    #[test]
    fn the_forces_page_draws_the_originals_layout() {
        // FUN_0046f360 / FUN_00471200 / FUN_004714e0 / FUN_004724a0.
        let entry = |name: &str, condition| BattleResultEntry {
            name: name.into(),
            condition,
            picture: Some((DllSource::Gokres, 0x640)),
            backdrop: Some((DllSource::Strategy, 11_514)),
        };
        let entries = [
            entry("Kept", BattleResultCondition::Operational),
            entry("Gone", BattleResultCondition::Destroyed),
            entry("Gone too", BattleResultCondition::Destroyed),
        ];
        let size = |_, id| Some(if id == 11_514 { [122, 50] } else { [100, 40] });
        let ops = battle_force_page(
            "Hoth",
            &entries,
            false,
            BattleResultCategory::Troops,
            true,
            WHITE,
            size,
        );
        let texts: Vec<(String, f32, f32, f32, bool)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Text { text, x, baseline, size, centered, .. } => {
                    Some((text.clone(), *x, *baseline, *size, *centered))
                }
                PageOp::Bitmap { .. } | PageOp::Wrapped { .. } => None,
            })
            .collect();
        let find = |name: &str| texts.iter().find(|entry| entry.0 == name).cloned().unwrap();
        assert_eq!(find("Battle at Hoth"), ("Battle at Hoth".into(), 212.0, 32.0, 18.0, true));
        assert_eq!(find("Imperial Forces"), ("Imperial Forces".into(), 212.0, 54.0, 14.0, true));
        assert_eq!(find("Filters"), ("Filters".into(), 86.0, 80.0, 10.0, false));
        assert_eq!(
            find("Trooper Regiments"),
            ("Trooper Regiments".into(), 37.0, 109.0, 10.0, false)
        );
        assert_eq!(find("Operational").1, 37.0 + 82.5);
        assert_eq!(find("Destroyed").1, 204.0 + 82.5);
        assert_eq!(find("Destroyed").2, 128.0);
        assert_eq!(find("Kept"), ("Kept".into(), 37.0 + 82.5, 136.0 + 70.0 - 14.0 + 9.0, 9.0, true));
        assert_eq!(find("Gone too").1, 202.0 + 82.5);
        assert_eq!(find("Gone too").2, 206.0 + 70.0 - 14.0 + 9.0);
        let bitmaps: Vec<(u32, f32, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Bitmap { id, x, y, .. } => Some((*id, *x, *y)),
                PageOp::Text { .. } | PageOp::Wrapped { .. } => None,
            })
            .collect();
        // The two-column art, the pressed Trooper Regiments button.
        assert_eq!(bitmaps[0], (10_761, 12.0, 13.0));
        assert!(bitmaps.contains(&(0x286f, 228.0, 59.0)));
        assert!(bitmaps.contains(&(0x286c, 130.0, 59.0)));
        // The backdrop and the picture centred over the name: (165 - 122)
        // / 2 = 21, (70 - 14 - 50) / 2 = 3; (165 - 100) / 2 = 32, (56 -
        // 40) / 2 = 8.
        assert!(bitmaps.contains(&(11_514, 37.0 + 21.0, 136.0 + 3.0)));
        assert!(bitmaps.contains(&(0x640, 37.0 + 32.0, 136.0 + 8.0)));
        assert!(bitmaps.contains(&(0x640, 202.0 + 32.0, 206.0 + 8.0)));
        // Personnel: three columns and the three-column art.
        let ops = battle_force_page(
            "Hoth",
            &[],
            true,
            BattleResultCategory::Personnel,
            false,
            WHITE,
            size,
        );
        assert!(ops.contains(&PageOp::Bitmap {
            source: DllSource::Strategy,
            id: 10_762,
            x: 12.0,
            y: 13.0
        }));
        let heads: Vec<(String, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Text { text, x, .. } if ["Survivors", "Captured", "Killed"].contains(&text.as_str()) => {
                    Some((text.clone(), *x))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            heads,
            [
                ("Survivors".into(), 92.0),
                ("Captured".into(), 204.0),
                ("Killed".into(), 315.0)
            ]
        );
        let labels: Vec<(String, f32, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Text { text, x, baseline, .. } if ["None", "No Casualties"].contains(&text.as_str()) => {
                    Some((text.clone(), *x, *baseline))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            // The list's 110-wide cells from x 37 (FUN_004714e0), not the
            // heads' x (FUN_0046f360 places those at 37, 149, 260).
            [
                ("None".into(), 92.0, 150.0),
                ("None".into(), 202.0, 150.0),
                ("No Casualties".into(), 312.0, 150.0)
            ]
        );
        assert!(ops.iter().any(|op| matches!(op, PageOp::Text { text, color, .. } if text == "Alliance Forces" && *color == Color::from_rgba(255, 0, 0, 255))));
    }

    /// TEXTSTRA RCDATA 0x7002..0x700f as shipped, and the sides'
    /// adjectives (0x4d70 / 0x4d71).
    fn battle_texts() -> HashMap<u16, String> {
        [
            (0x7002, "The {1:4} fleet is victorious. "),
            (0x7003, "The {1:4} fleet is defeated. "),
            (0x7004, "The battle at {1:1} is indecisive. "),
            (0x7005, "{1:1} has been successfully defended from {2:4} forces. "),
            (0x7006, "{1:1} is now under blockade by {2:4} forces. "),
            (0x7007, "{1:1} has been cleared of {2:4} forces. "),
            (0x7008, "{1:1} is now under blockade by {2:4} forces. "),
            (0x7009, "The {2:4} attack on {1:1} has failed. "),
            (0x700a, "The {2:4} fleet was victorious at the neutral world {1:1}. "),
            (0x700b, "The {1:4} fleet has been completely destroyed. "),
            (0x700c, "The {1:4} fleet has withdrawn. "),
            (0x700e, "There has been no victor.  "),
            (0x700f, "All {1:4} and {2:4} ships have been destroyed. "),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect()
    }

    #[test]
    fn the_session_gives_the_report_its_withdrawn_side_and_system() {
        // FUN_0049b060: +0x54 is the side whose fleet withdrew; the system's
        // side bits pick the sentence (port: from the world's control).
        let mut fled = test_ship(64, 1, true, true);
        fled.retreated = true;
        let stayed = test_ship(64, 2, false, true);
        let mut session = test_session(vec![fled, stayed], Vec::new(), true);
        session.winner = Some(CombatWinner::Defender);
        let world = GameWorld::default();
        let lines = battle_result_summary_lines(&session, &world);
        // The texts are not loaded in tests: the port's English names the
        // withdrawal.
        assert!(lines.iter().any(|line| line.contains("withdrawn")), "{lines:?}");
        session.ships[0].retreated = false;
        session.ships[1].retreated = true;
        let lines = battle_result_summary_lines(&session, &world);
        assert!(lines.iter().any(|line| line.contains("withdrawn")), "{lines:?}");
        session.ships[1].retreated = false;
        let lines = battle_result_summary_lines(&session, &world);
        assert!(lines.iter().any(|line| line.contains("completely destroyed")), "{lines:?}");
    }

    #[test]
    fn the_app_hands_the_report_its_texts() {
        // set_battle_texts: the report then reads TEXTSTRA. With no winner
        // the withdrawn side matters: here the Alliance's (the viewer's,
        // as the attacker) withdrew (FUN_0049b150).
        set_battle_texts(battle_texts());
        let mut fled = test_ship(64, 1, true, true);
        fled.retreated = true;
        let session = test_session(vec![fled, test_ship(64, 2, false, true)], Vec::new(), true);
        let lines = battle_result_summary_lines(&session, &GameWorld::default());
        assert_eq!(lines[0], "The battle at Test System is indecisive.");
        assert_eq!(
            lines[1],
            "There has been no victor.  The Alliance fleet has withdrawn. The Imperial fleet has been completely destroyed."
        );
    }

    #[test]
    fn the_battle_report_speaks_for_the_viewer() {
        // FUN_0049b0f0: the viewer's win (FUN_0049b330), loss
        // (FUN_0049b4d0) or no winner (FUN_0049b150); guide-059 reads "The
        // Imperial fleet is victorious." then "Deyer is now under blockade
        // by Imperial forces. The Alliance fleet has been completely
        // destroyed."
        let texts = battle_texts();
        let report = |viewer, winner, withdrawn, system_side| {
            battle_report_lines(
                &texts,
                &BattleReport {
                    system_name: "Deyer",
                    viewer_alliance: viewer,
                    winner_alliance: winner,
                    withdrawn,
                    system_side,
                },
            )
        };
        assert_eq!(
            report(false, Some(false), None, Some(true)),
            [
                "The Imperial fleet is victorious.",
                "Deyer is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed."
            ]
        );
        // The same battle for the Alliance viewer.
        assert_eq!(
            report(true, Some(false), None, Some(true)),
            [
                "The Alliance fleet is defeated.",
                "Deyer is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed."
            ]
        );
        // A win at home: defended; at a neutral world: cleared; the loser
        // withdrew.
        assert_eq!(
            report(true, Some(true), Some(false), Some(true))[1],
            "Deyer has been successfully defended from Imperial forces. The Imperial fleet has withdrawn."
        );
        assert_eq!(
            report(true, Some(true), None, None)[1],
            "Deyer has been cleared of Imperial forces. The Imperial fleet has been completely destroyed."
        );
        // A loss: a failed attack on the enemy's system, at a neutral world.
        assert_eq!(
            report(true, Some(false), Some(true), Some(false))[1],
            "The Alliance attack on Deyer has failed. The Alliance fleet has withdrawn."
        );
        assert_eq!(
            report(true, Some(false), None, None)[1],
            "The Imperial fleet was victorious at the neutral world Deyer. The Alliance fleet has been completely destroyed."
        );
        // No winner.
        assert_eq!(
            report(true, None, None, Some(true)),
            [
                "The battle at Deyer is indecisive.",
                "There has been no victor.  All Alliance and Imperial ships have been destroyed."
            ]
        );
        assert_eq!(
            report(true, None, Some(true), Some(true))[1],
            "There has been no victor.  The Alliance fleet has withdrawn. The Imperial fleet has been completely destroyed."
        );
        assert_eq!(
            report(true, None, Some(false), Some(true))[1],
            "There has been no victor.  The Alliance fleet has been completely destroyed. The Imperial fleet has withdrawn."
        );
        // Without the texts, the port's English.
        let empty = HashMap::new();
        let fallback = battle_report_lines(
            &empty,
            &BattleReport {
                system_name: "Deyer",
                viewer_alliance: true,
                winner_alliance: Some(false),
                withdrawn: Some(true),
                system_side: None,
            },
        );
        assert_eq!(fallback[0], "The Imperial fleet is victorious.");
        assert_eq!(fallback[2], "The Alliance fleet has withdrawn from the system.");
        let none = battle_report_lines(
            &empty,
            &BattleReport {
                system_name: "Deyer",
                viewer_alliance: true,
                winner_alliance: None,
                withdrawn: None,
                system_side: None,
            },
        );
        assert_eq!(none[0], "Neither fleet is victorious.");
    }

    #[test]
    fn the_battle_alert_tells_the_viewer_who_is_moving_on_whom() {
        // FUN_0049c0d0 with TEXTSTRA RCDATA 0x7020..0x7025 as shipped.
        let mut texts = battle_texts();
        for (id, text) in [
            (0x7020, "Battle at {1:1}"),
            (0x7021, "The {1:4} fleet is threatening {2:1}. {3:4} forces are moving to intercept. "),
            (0x7022, "The {1:4} fleet is attempting to break the {3:4} blockade of {2:4}. "),
            (0x7023, "The {1:4} fleet has entered the {2:1} system. {3:4} forces have been detected on an intercept course. "),
            (0x7024, "The {1:4} fleet is attempting to break the {2:4} blockade at {3:1}"),
            (0x7025, "{1:4} and {2:4} forces are about to engage in battle near the {3:1} system. "),
        ] {
            texts.insert(id, text.to_owned());
        }
        // guide-055: the Empire's viewer at the Alliance's Deyer, where the
        // Empire arrived last.
        assert_eq!(
            battle_alert_lines(&texts, false, "Deyer", Some(true), Some(false)),
            [
                "Battle at Deyer",
                "The Imperial fleet has entered the Deyer system. Alliance forces have been detected on an intercept course."
            ]
        );
        // The Alliance's viewer of the same battle: its system is threatened.
        assert_eq!(
            battle_alert_lines(&texts, true, "Deyer", Some(true), Some(false))[1],
            "The Imperial fleet is threatening Deyer. Alliance forces are moving to intercept."
        );
        // The viewer arrived last at its own system: it breaks a blockade.
        assert_eq!(
            battle_alert_lines(&texts, true, "Deyer", Some(true), Some(true))[1],
            "The Alliance fleet is attempting to break the Imperial blockade of Deyer."
        );
        // The enemy arrived last at its own system.
        assert_eq!(
            battle_alert_lines(&texts, true, "Deyer", Some(false), Some(false))[1],
            "The Imperial fleet is attempting to break the Alliance blockade at Deyer"
        );
        assert_eq!(
            battle_alert_lines(&texts, false, "Deyer", None, Some(true))[1],
            "Imperial and Alliance forces are about to engage in battle near the Deyer system."
        );
        // Without the texts, the port's English says the same.
        let empty = HashMap::new();
        for (viewer, side, last) in [
            (false, Some(true), Some(false)),
            (true, Some(true), Some(false)),
            (true, Some(true), Some(true)),
            (true, Some(false), Some(false)),
            (false, None, Some(true)),
        ] {
            assert_eq!(
                battle_alert_lines(&empty, viewer, "Deyer", side, last),
                battle_alert_lines(&texts, viewer, "Deyer", side, last)
            );
        }
    }

    fn test_system(
        world: &mut GameWorld,
        control: rebellion_core::world::ControlKind,
        last_arrival: Option<rebellion_core::dat::Faction>,
    ) -> SystemKey {
        world.systems.insert(rebellion_core::world::System {
            last_arrival,
            dat_id: DatId::new(0x9000_0001),
            name: "Deyer".into(),
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
            fighters: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control,
        })
    }

    #[test]
    fn the_battle_alert_lists_a_side_s_fleets_then_its_planet_squadrons() {
        // FUN_00450770: the heading (0x1923), each fleet's name, its ships
        // (FUN_00450e10 font 5 with the mini) and the units aboard (font 4,
        // indented); then the system's name and the side's squadrons on the
        // planet (FUN_00503a50). guide-057 shows "Alliance Forces",
        // "Deyer", "A-wing".
        use rebellion_core::world::{
            CapitalShipClass, Character, ControlKind, FighterClass, FighterEntry, Fleet,
            ShipInstance,
        };
        let mut world = GameWorld::default();
        let system = test_system(&mut world, ControlKind::Uncontrolled, None);
        let corvette = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            name: "Corellian Corvette".into(),
            is_alliance: true,
            ..CapitalShipClass::default()
        });
        let awing = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            name: "A-wing".into(),
            is_alliance: true,
            ..FighterClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x1c00_0005),
            name: "TIE Fighter".into(),
            is_alliance: false,
            ..FighterClass::default()
        });
        let leia = world.characters.insert(Character {
            name: "Leia".into(),
            ..Default::default()
        });
        let fleet = |is_alliance, capital_ships, fighters, characters| Fleet {
            location: system,
            capital_ships,
            fighters,
            characters,
            is_alliance,
            has_death_star: false,
        };
        let rebels = world.fleets.insert(fleet(
            true,
            vec![ShipInstance::new(corvette, 0, false)],
            vec![FighterEntry {
                class: awing,
                count: 1,
            }],
            vec![leia],
        ));
        let empire = world.fleets.insert(fleet(false, vec![], vec![], vec![]));
        world.systems[system].fleets = vec![empire, rebels];
        world.systems[system].fighters = vec![
            FighterEntry {
                class: awing,
                count: 2,
            },
            FighterEntry {
                class: tie,
                count: 1,
            },
        ];
        let rows = battle_alert_rows(
            &world,
            &TroopTransportState::default(),
            system,
            BattleAlertList::Forces { alliance: true },
        );
        let summary: Vec<(&str, AlertFont, bool, bool)> = rows
            .iter()
            .map(|row| {
                (
                    row.text.as_str(),
                    row.font,
                    !row.picture.is_empty(),
                    row.indent,
                )
            })
            .collect();
        let name = world.fleet_name(rebels).unwrap_or_default().to_owned();
        assert_eq!(
            summary,
            [
                ("Alliance Forces", AlertFont::Heading, false, false),
                (name.as_str(), AlertFont::Bold, false, false),
                ("Corellian Corvette", AlertFont::Bold, true, false),
                ("A-wing", AlertFont::Plain, true, true),
                ("Leia", AlertFont::Plain, false, true),
                ("Deyer", AlertFont::Bold, false, false),
                ("A-wing", AlertFont::Plain, true, false),
                ("A-wing", AlertFont::Plain, true, false),
            ]
        );
        // The ship's mini is keyed over nothing; the squadron's too.
        assert_eq!(
            rows[2].picture,
            [(
                DllSource::Gokres,
                crate::panels::fleets::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap()
            )]
        );
        // The Empire's tab: its empty fleet's name, its TIE on the planet.
        let rows = battle_alert_rows(
            &world,
            &TroopTransportState::default(),
            system,
            BattleAlertList::Forces { alliance: false },
        );
        let texts: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
        let name = world.fleet_name(empire).unwrap_or_default().to_owned();
        assert_eq!(
            texts,
            ["Imperial Forces", name.as_str(), "Deyer", "TIE Fighter"]
        );
    }

    #[test]
    fn the_battle_alert_lists_the_system_assets() {
        // FUN_00450770(0): the heading 0x1925, the facilities
        // (FUN_00539d70), the regiments (FUN_00504c40) and the personnel
        // (FUN_00536da0) on the planet, font 4, not indented. guide-056
        // lists "LNR Series II" and "GenCore Level II".
        use rebellion_core::dat::Faction;
        use rebellion_core::world::{Character, ControlKind, DefenseFacilityInstance, TroopUnit};
        let mut world = GameWorld::default();
        let system = test_system(&mut world, ControlKind::Controlled(Faction::Alliance), None);
        let lnr = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2200_0005),
            side: Faction::Alliance,
        });
        let army = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0002),
            is_alliance: true,
            regiment_strength: 1,
        });
        world.systems[system].defense_facilities = vec![lnr];
        world.systems[system].ground_units = vec![army];
        world.characters.insert(Character {
            name: "Mon Mothma".into(),
            current_system: Some(system),
            ..Default::default()
        });
        world.characters.insert(Character {
            name: "Elsewhere".into(),
            ..Default::default()
        });
        let rows = battle_alert_rows(
            &world,
            &TroopTransportState::default(),
            system,
            BattleAlertList::SystemAssets,
        );
        let texts: Vec<(&str, AlertFont, bool)> = rows
            .iter()
            .map(|row| (row.text.as_str(), row.font, row.indent))
            .collect();
        assert_eq!(
            texts,
            [
                ("System Assets", AlertFont::Heading, false),
                ("LNR Series II", AlertFont::Plain, false),
                ("Alliance Army Regiment", AlertFont::Plain, false),
                ("Mon Mothma", AlertFont::Plain, false),
            ]
        );
        // A regiment's mini sits on its backdrop, STRATEGY 11513.
        assert_eq!(
            rows[2].picture,
            [(DllSource::Strategy, 11_513), (DllSource::Gokres, 17_473)]
        );
    }

    #[test]
    fn each_battle_alert_tab_fills_its_own_list() {
        // FUN_00450660: tab 1 hides the list; 2, 3, 4 fill 1, 2, 0.
        assert_eq!(BattleAlertList::of(BattleAlertTab::Summary), None);
        assert_eq!(
            BattleAlertList::of(BattleAlertTab::AllianceForces),
            Some(BattleAlertList::Forces { alliance: true })
        );
        assert_eq!(
            BattleAlertList::of(BattleAlertTab::ImperialForces),
            Some(BattleAlertList::Forces { alliance: false })
        );
        assert_eq!(
            BattleAlertList::of(BattleAlertTab::System),
            Some(BattleAlertList::SystemAssets)
        );
    }

    #[test]
    fn an_injured_character_in_the_battle_alert_wears_the_damage_mark() {
        // FUN_004f7800: injury != 0 is damaged; FUN_0042c3b0 puts STRATEGY
        // 11502 over the backdrop 11500 for personnel.
        use rebellion_core::world::{Character, ControlKind};
        let mut world = GameWorld::default();
        let system = test_system(&mut world, ControlKind::Uncontrolled, None);
        let mut person = |injury| {
            world.characters.insert(Character {
                name: "Luke".into(),
                dat_id: DatId::new(0x3000_0240),
                is_major: true,
                current_system: Some(system),
                injury,
                ..Default::default()
            })
        };
        person(0);
        person(5);
        let rows = battle_alert_rows(
            &world,
            &TroopTransportState::default(),
            system,
            BattleAlertList::SystemAssets,
        );
        let mini =
            rebellion_core::pictures::character_mini_resource_id(DatId::new(0x3000_0240), true)
                .unwrap();
        assert_eq!(
            rows[1].picture,
            [(DllSource::Strategy, 11_500), (DllSource::Gokres, mini)]
        );
        assert_eq!(
            rows[2].picture,
            [
                (DllSource::Strategy, 11_500),
                (DllSource::Strategy, 11_502),
                (DllSource::Gokres, mini)
            ]
        );
    }

    #[test]
    fn the_battle_alert_list_rows_are_forty_high_from_83_40() {
        // FUN_0044f860: FUN_00607ea0(.., 0x53, 0x28, 0x100, 0xea) with
        // rows 0x28 high; FUN_00450e10: text at 0x46 past a picture, 0x5a
        // when indented by 0x14.
        let text = |text: &str, font| AlertRow {
            text: text.into(),
            font,
            picture: Vec::new(),
            indent: false,
        };
        let picture = |indent| AlertRow {
            text: "A-wing".into(),
            font: AlertFont::Plain,
            picture: vec![(DllSource::Gokres, 7)],
            indent,
        };
        let mut rows = vec![
            text("Alliance Forces", AlertFont::Heading),
            picture(false),
            picture(true),
        ];
        rows.extend((0..5).map(|_| text("more", AlertFont::Bold)));
        let ops = battle_alert_list(&rows);
        assert_eq!(
            ops[0],
            PageOp::Text {
                text: "Alliance Forces".into(),
                x: 211.0,
                baseline: 67.0,
                size: 18.0,
                color: WHITE,
                centered: true,
            }
        );
        assert_eq!(
            ops[1],
            PageOp::Bitmap {
                source: DllSource::Gokres,
                id: 7,
                x: 83.0,
                y: 80.0
            }
        );
        assert!(
            matches!(&ops[2], PageOp::Text { x, baseline, centered: false, .. } if *x == 153.0 && *baseline == 105.0)
        );
        assert_eq!(
            ops[3],
            PageOp::Bitmap {
                source: DllSource::Gokres,
                id: 7,
                x: 103.0,
                y: 120.0
            }
        );
        assert!(matches!(&ops[4], PageOp::Text { x, .. } if *x == 173.0));
        // 234 high: the sixth row (top 240, middle 260) is the last whose
        // middle fits; the seventh is cut.
        let texts = ops
            .iter()
            .filter(|op| matches!(op, PageOp::Text { .. }))
            .count();
        assert_eq!(texts, 6);
    }

    #[test]
    fn the_battle_alert_reads_the_system_side_and_last_arrival() {
        // FUN_0049c0d0: +0x24 bits 6..7 and +0x78 bits 6..7.
        use rebellion_core::dat::Faction;
        use rebellion_core::world::ControlKind;
        let mut world = GameWorld::default();
        let mut system = |control, last_arrival| test_system(&mut world, control, last_arrival);
        let rebel = system(
            ControlKind::Controlled(Faction::Alliance),
            Some(Faction::Empire),
        );
        let imperial = system(
            ControlKind::Controlled(Faction::Empire),
            Some(Faction::Alliance),
        );
        let neutral = system(ControlKind::Uncontrolled, None);
        assert_eq!(battle_alert_sides(&world, rebel), (Some(true), Some(false)));
        assert_eq!(
            battle_alert_sides(&world, imperial),
            (Some(false), Some(true))
        );
        assert_eq!(battle_alert_sides(&world, neutral), (None, None));
        assert_eq!(
            battle_alert_sides(&GameWorld::default(), rebel),
            (None, None)
        );
    }

    #[test]
    fn the_display_page_tells_the_viewer_where_its_fleet_is() {
        // FUN_0049b330 / FUN_0049b4d0 / FUN_0049b150: +0xac reads 0x7012
        // on the viewer's win, 0x7011 on its withdrawal (port: without
        // the destination), else 0x7010; +0xb8 reads 0x7013 for two
        // buttons, 0x7014 for one. TEXTSTRA as shipped.
        let mut texts = battle_texts();
        for (id, text) in [
            (
                0x7010,
                "The {1:4} fleet was defeated at {2:1}. Your fleet has been destroyed. ",
            ),
            (
                0x7011,
                "The {1:4} fleet was defeated at {2:1}. Your fleet has withdrawn to {3:1}. ",
            ),
            (
                0x7012,
                "The {1:4} fleet is victorious. The world {2:1} is now under {1:4} blockade. ",
            ),
            (
                0x7013,
                "Select one of the following buttons to close this display and go directly to... ",
            ),
            (
                0x7014,
                "Select the following button to close this display and go directly to... ",
            ),
        ] {
            texts.insert(id, text.to_owned());
        }
        let display = |texts: &HashMap<u16, String>, winner, withdrawn| {
            battle_display_lines(
                texts,
                &BattleReport {
                    system_name: "Deyer",
                    viewer_alliance: false,
                    winner_alliance: winner,
                    withdrawn,
                    system_side: None,
                },
            )
        };
        let two = "Select one of the following buttons to close this display and go directly to...";
        let one = "Select the following button to close this display and go directly to...";
        assert_eq!(
            display(&texts, Some(false), Some(true)),
            (
                [
                    "The Imperial fleet is victorious. The world Deyer is now under Imperial blockade.".to_owned(),
                    two.to_owned()
                ],
                true
            )
        );
        assert_eq!(
            display(&texts, Some(true), Some(false)),
            (
                [
                    "The Imperial fleet was defeated at Deyer. Your fleet has withdrawn."
                        .to_owned(),
                    two.to_owned()
                ],
                true
            )
        );
        // No winner, the viewer withdrew: its fleet still has a button.
        assert!(display(&texts, None, Some(false)).1);
        let lost = (
            [
                "The Imperial fleet was defeated at Deyer. Your fleet has been destroyed."
                    .to_owned(),
                one.to_owned(),
            ],
            false,
        );
        assert_eq!(display(&texts, Some(true), None), lost);
        // The enemy's withdrawal leaves the viewer's fleet destroyed.
        assert_eq!(display(&texts, None, Some(true)), lost);
        // Without the texts, the port's English says the same.
        assert_eq!(display(&HashMap::new(), Some(true), None), lost);
        assert_eq!(display(&HashMap::new(), Some(false), None).0[1], two);
        assert_eq!(
            display(&HashMap::new(), None, Some(false)).0[0],
            "The Imperial fleet was defeated at Deyer. Your fleet has withdrawn."
        );
    }

    #[test]
    fn the_display_page_lays_out_its_buttons_like_the_original() {
        // FUN_0046f360: the lines at (24, 40) and (24, 130), 328 wide; the
        // system button 0xcb at (29, 182) with 0x2a0c, the fleet button
        // 0xca at (229, 182) with 0x2a0d / 0x2a0e by the viewer, pressed
        // 0x2a0f / 0x2a10; the labels at y 278, the fleet's 0x1942.
        let lines = ["message".to_owned(), "instruction".to_owned()];
        let red = Color::from_rgba(255, 0, 0, 255);
        let ops = battle_display_page("Deyer", &lines, true, true, Some(DisplayButton::Fleet), red);
        let bitmaps: Vec<(u32, f32, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Bitmap { id, x, y, .. } => Some((*id, *x, *y)),
                _ => None,
            })
            .collect();
        assert_eq!(
            bitmaps,
            [
                (10763, 12.0, 13.0),
                (10764, 29.0, 182.0),
                (10767, 229.0, 182.0)
            ]
        );
        let wrapped: Vec<(f32, f32, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Wrapped {
                    x,
                    baseline,
                    width,
                    color,
                    ..
                } => {
                    assert_eq!(*color, red);
                    Some((*x, *baseline, *width))
                }
                _ => None,
            })
            .collect();
        assert_eq!(wrapped, [(24.0, 52.0, 328.0), (24.0, 142.0, 328.0)]);
        let labels: Vec<(&str, f32, f32)> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Text {
                    text,
                    x,
                    baseline,
                    centered: true,
                    color,
                    ..
                } => {
                    assert_eq!(*color, WHITE);
                    Some((text.as_str(), *x, *baseline))
                }
                _ => None,
            })
            .collect();
        assert_eq!(labels, [("Deyer", 113.5, 292.0), ("Fleet", 313.5, 292.0)]);
        // The Empire's viewer: its fleet art and pressed system art.
        let ops = battle_display_page(
            "Deyer",
            &lines,
            false,
            true,
            Some(DisplayButton::System),
            red,
        );
        assert!(ops.contains(&PageOp::Bitmap {
            source: DllSource::Strategy,
            id: 10768,
            x: 29.0,
            y: 182.0
        }));
        assert!(ops.contains(&PageOp::Bitmap {
            source: DllSource::Strategy,
            id: 10766,
            x: 229.0,
            y: 182.0
        }));
        // No fleet: one button.
        let ops = battle_display_page("Deyer", &lines, false, false, None, red);
        assert!(!ops
            .iter()
            .any(|op| matches!(op, PageOp::Bitmap { x, .. } if *x == 229.0)));
        assert!(!ops
            .iter()
            .any(|op| matches!(op, PageOp::Text { text, .. } if text == "Fleet")));
    }

    #[test]
    fn the_display_buttons_answer_inside_their_rectangles() {
        assert_eq!(
            display_button_at(29.0, 182.0, false),
            Some(DisplayButton::System)
        );
        assert_eq!(
            display_button_at(197.9, 277.9, true),
            Some(DisplayButton::System)
        );
        assert_eq!(display_button_at(198.0, 200.0, true), None);
        assert_eq!(display_button_at(28.9, 200.0, true), None);
        assert_eq!(display_button_at(100.0, 181.9, true), None);
        assert_eq!(display_button_at(100.0, 278.0, true), None);
        assert_eq!(
            display_button_at(229.0, 182.0, true),
            Some(DisplayButton::Fleet)
        );
        assert_eq!(
            display_button_at(397.9, 277.9, true),
            Some(DisplayButton::Fleet)
        );
        assert_eq!(display_button_at(300.0, 200.0, false), None);
    }

    #[test]
    fn the_summary_page_heads_the_outcome_over_its_shadowed_lines() {
        // FUN_0046f360 (guide-059): the title at (12, 18) and the first
        // line at (12, 225), 400 wide, centred; the rest at (24, 255), 352
        // wide, over a black shadow at (25, 256).
        let lines = vec![
            "The Imperial fleet is victorious.".to_owned(),
            "Hoth is now under blockade by Imperial forces.".to_owned(),
            "The Alliance fleet has been completely destroyed.".to_owned(),
        ];
        let green = Color::from_rgba(0, 255, 0, 255);
        let ops = battle_summary_page("Hoth", Some(false), &lines, green);
        assert_eq!(
            ops[0],
            PageOp::Bitmap {
                source: DllSource::Strategy,
                id: resources::strategy::EVENT_BATTLE_EMPIRE_VICTORY,
                x: 12.0,
                y: 13.0
            }
        );
        assert_eq!(
            ops[2],
            PageOp::Text {
                text: lines[0].clone(),
                x: 212.0,
                baseline: 241.0,
                size: 18.0,
                color: green,
                centered: true
            }
        );
        assert_eq!(
            ops[3],
            PageOp::Wrapped {
                lines: lines[1..].to_vec(),
                x: 25.0,
                baseline: 268.0,
                width: 352.0,
                size: 12.0,
                color: BLACK
            }
        );
        assert_eq!(
            ops[4],
            PageOp::Wrapped {
                lines: lines[1..].to_vec(),
                x: 24.0,
                baseline: 267.0,
                width: 352.0,
                size: 12.0,
                color: green
            }
        );
        assert!(matches!(&ops[1], PageOp::Text { text, baseline, .. } if text == "Battle at Hoth" && *baseline == 34.0));
        // No outcome, no lines; a draw shows the mutual destruction scene.
        let ops = battle_summary_page("Hoth", None, &[], green);
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], PageOp::Bitmap { id, .. } if id == resources::strategy::EVENT_BATTLE_MUTUAL_DESTRUCTION));
        let ops = battle_summary_page("Hoth", Some(true), &[], green);
        assert!(matches!(ops[0], PageOp::Bitmap { id, .. } if id == resources::strategy::EVENT_BATTLE_ALLIANCE_VICTORY));
    }

    #[test]
    fn the_battle_summary_category_bar_follows_the_original() {
        // FUN_0046f360: the bar at (130, 59), 49 by 41 buttons, the
        // viewer's art; the labels TEXTSTRA 0x1928..0x1931.
        assert_eq!(battle_category_rect(0), NativeRect::new(130.0, 59.0, 49.0, 41.0));
        assert_eq!(battle_category_rect(3), NativeRect::new(277.0, 59.0, 49.0, 41.0));
        assert_eq!(
            battle_category_art(BattleResultCategory::CapitalShips, true),
            (0x286c, 0x286b)
        );
        assert_eq!(
            battle_category_art(BattleResultCategory::Fighters, false),
            (0x287c, 0x287b)
        );
        assert_eq!(
            battle_category_art(BattleResultCategory::Troops, true),
            (0x2870, 0x286f)
        );
        assert_eq!(
            battle_category_art(BattleResultCategory::Personnel, false),
            (0x2876, 0x2875)
        );
        assert_eq!(BattleResultCategory::Troops.title(), "Trooper Regiments");
        assert_eq!(BattleResultCategory::Fighters.title(), "Fighter Squadrons");
    }

    #[test]
    fn auto_resolve_enters_results_and_projects_losses_once() {
        let mut first_ship = test_ship(64, 1, true, true);
        first_ship.hull_current = 100;
        first_ship.hull_max = 100;
        let mut second_ship = test_ship(64, 2, true, true);
        second_ship.hull_current = 100;
        second_ship.hull_max = 100;
        let mut fighters = vec![test_fighter(1, true), test_fighter(1, true)];
        fighters[1].fleet_squadron_index = 1;
        let session = test_session(vec![first_ship, second_ship], fighters, true);
        let attacker_fleet = session.attacker_fleet;
        let defender_fleet = session.defender_fleet;
        let system = session.system;
        let mut state = TacticalState {
            session: Some(session),
            ..TacticalState::default()
        };
        let result = SpaceCombatResult {
            attacker_fleet,
            defender_fleet,
            system,
            winner: AutoResolveSide::Attacker,
            ship_damage: vec![rebellion_core::combat::ShipDamageEvent {
                fleet: attacker_fleet,
                ship_index: 0,
                hull_before: 100,
                hull_after: 0,
                hits_after: 0,
            }],
            fighter_losses: vec![rebellion_core::combat::FighterLossEvent {
                fleet: attacker_fleet,
                fighter_index: 0,
                squads_before: 2,
                squads_after: 1,
            }],
            tick: 0,
        };

        assert!(state.present_auto_resolve_result(&result));
        assert!(state.strategic_results_applied());
        let result_session = state.session.as_mut().unwrap();
        assert_eq!(
            result_session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_won(
                TacticalVoiceFaction::Alliance,
                false
            )]
        );
        assert_eq!(result_session.phase, BattlePhase::Results);
        assert_eq!(result_session.winner, Some(CombatWinner::Attacker));
        assert!(!result_session.ships[0].alive);
        assert!(result_session.ships[1].alive);
        assert!(result_session.fighters[0].alive);
        assert!(!result_session.fighters[1].alive);
    }

    // Source: FUN_005d3e90 effect dispatcher; FUN_005a7500 hit ordering.
    #[test]
    fn original_effect_dispatch_preserves_resources_sizes_and_priority_groups() {
        let cases = [
            (
                WeaponKind::LaserCannon,
                OriginalTacticalImpactStage::Hit,
                OriginalTacticalEffectSequence::StandardHit,
                3060,
                6,
                (32, 32),
            ),
            (
                WeaponKind::IonCannon,
                OriginalTacticalImpactStage::Hit,
                OriginalTacticalEffectSequence::IonHit,
                3180,
                6,
                (32, 32),
            ),
            (
                WeaponKind::LaserCannon,
                OriginalTacticalImpactStage::Damage,
                OriginalTacticalEffectSequence::StandardDamage,
                3120,
                7,
                (32, 32),
            ),
            (
                WeaponKind::IonCannon,
                OriginalTacticalImpactStage::Damage,
                OriginalTacticalEffectSequence::IonDamage,
                3240,
                16,
                (64, 32),
            ),
            (
                WeaponKind::Turbolaser,
                OriginalTacticalImpactStage::Damage,
                OriginalTacticalEffectSequence::TurbolaserDamage,
                3300,
                7,
                (32, 32),
            ),
            (
                WeaponKind::LaserCannon,
                OriginalTacticalImpactStage::Destroyed,
                OriginalTacticalEffectSequence::Destroyed,
                3360,
                16,
                (64, 64),
            ),
        ];
        for (weapon, stage, expected, base, frames, size) in cases {
            let actual = original_tactical_effect_sequence(weapon, stage);
            assert_eq!(actual, expected);
            assert_eq!(actual.resource_base(), base);
            assert_eq!(actual.frame_count(), frames);
            assert_eq!(actual.source_size(), size);
        }

        let mut effects = Vec::new();
        assert!(queue_original_tactical_impact(
            &mut effects,
            4,
            WeaponKind::IonCannon,
            OriginalTacticalImpactStage::Hit,
        ));
        assert!(!queue_original_tactical_impact(
            &mut effects,
            4,
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Hit,
        ));
        assert!(queue_original_tactical_impact(
            &mut effects,
            4,
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Damage,
        ));
        assert!(!queue_original_tactical_impact(
            &mut effects,
            4,
            WeaponKind::Turbolaser,
            OriginalTacticalImpactStage::Damage,
        ));
        assert!(queue_original_tactical_impact(
            &mut effects,
            4,
            WeaponKind::IonCannon,
            OriginalTacticalImpactStage::Destroyed,
        ));
        assert_eq!(
            effects[0].sequence,
            OriginalTacticalEffectSequence::Destroyed
        );
    }

    // Source: _DAT_0066d154 / FUN_005d3cc0 effect frame timing 0.1s.
    #[test]
    fn original_effect_frames_advance_at_ten_hertz_and_expire_exactly() {
        let mut session = test_session(Vec::new(), Vec::new(), true);
        queue_original_tactical_impact(
            &mut session.impact_effects,
            0,
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Hit,
        );
        session.advance_presentational_effects(0.099);
        assert_eq!(session.impact_effects[0].frame, 0);
        session.advance_presentational_effects(0.001_1);
        assert_eq!(session.impact_effects[0].frame, 1);
        session.advance_presentational_effects(0.5);
        assert!(session.impact_effects.is_empty());
    }

    // Source: FUN_005d3de0 / FUN_005b1ea0 / FUN_005b1f60 projectile profiles.
    #[test]
    fn original_projectile_profiles_preserve_thresholds_shapes_scales_colors_and_durations() {
        let cases = [
            (
                WeaponKind::LaserCannon,
                28.7,
                Some(0),
                true,
                (0.5, 1, 0, 1.0),
            ),
            (
                WeaponKind::LaserCannon,
                ORIGINAL_LASER_PROJECTILE_THRESHOLD,
                Some(0),
                true,
                (0.5, 2, 0, 1.0),
            ),
            (
                WeaponKind::Turbolaser,
                30.0,
                Some(15),
                false,
                (0.65, 1, 1, 1.0),
            ),
            (
                WeaponKind::Turbolaser,
                40.0,
                Some(14),
                false,
                (0.75, 2, 1, 1.0),
            ),
            (
                WeaponKind::Turbolaser,
                40.0,
                Some(15),
                false,
                (1.0, 3, 1, 2.0),
            ),
            (
                WeaponKind::IonCannon,
                ORIGINAL_ION_PROJECTILE_THRESHOLD,
                Some(0),
                true,
                (1.0, 3, 2, 2.0),
            ),
            (
                WeaponKind::FighterAttack,
                10.0,
                None,
                true,
                (0.2, 1, 0, 1.0),
            ),
        ];
        for (weapon, strength, ordinal, alliance, expected) in cases {
            let profile = original_projectile_profile(weapon, strength, ordinal, alliance);
            assert_eq!(
                (
                    profile.longitudinal_scale,
                    profile.shape_variant,
                    profile.color_selector,
                    profile.duration,
                ),
                expected
            );
        }
    }

    // Source: FUN_005ee590 / LAB_005eeb90 projectile interpolation.
    #[test]
    fn retained_projectile_interpolates_and_expires_by_source_duration() {
        let ships = vec![test_ship(64, 0, true, true), test_ship(128, 0, false, true)];
        let mut session = test_session(ships, Vec::new(), true);
        assert!(queue_original_tactical_projectile(
            &mut session.weapon_effects,
            &session.ships,
            0,
            1,
            WeaponKind::LaserCannon,
            ORIGINAL_LASER_PROJECTILE_THRESHOLD,
        ));
        assert_eq!(session.weapon_effects[0].shape_variant, 2);
        session.advance_presentational_effects(0.5);
        assert_eq!(session.weapon_effects[0].progress(), 0.5);
        session.advance_presentational_effects(0.499);
        assert_eq!(session.weapon_effects.len(), 1);
        session.advance_presentational_effects(0.002);
        assert!(session.weapon_effects.is_empty());
    }

    // Source: FUN_005b24d0 through FUN_005b2550 field effects.
    #[test]
    fn gravity_field_preempts_tractor_then_restores_it_at_ten_hertz() {
        let mut session = test_session(Vec::new(), Vec::new(), true);
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            1,
            4,
            OriginalTacticalFieldKind::Tractor,
            true,
        ));
        assert_eq!(session.field_effects[0].resource_id(), Some(3620));
        session.advance_presentational_effects(0.1);
        assert_eq!(session.field_effects[0].resource_id(), Some(3621));
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            2,
            4,
            OriginalTacticalFieldKind::Gravity,
            true,
        ));
        assert_eq!(session.field_effects[0].resource_id(), Some(3520));
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            3,
            4,
            OriginalTacticalFieldKind::Tractor,
            true,
        ));
        assert_eq!(session.field_effects[0].source_counts(), (2, 1));
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            2,
            4,
            OriginalTacticalFieldKind::Gravity,
            false,
        ));
        assert_eq!(session.field_effects[0].resource_id(), Some(3620));
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            1,
            4,
            OriginalTacticalFieldKind::Tractor,
            false,
        ));
        assert!(set_original_tactical_field(
            &mut session.field_effects,
            3,
            4,
            OriginalTacticalFieldKind::Tractor,
            false,
        ));
        assert!(session.field_effects.is_empty());
    }

    // Source: FUN_005b24d0 through FUN_005b2550 field source limits.
    #[test]
    fn exact_field_sources_enforce_one_tractor_and_four_gravity_targets() {
        let ships = (0..6)
            .map(|index| test_ship(64, index, true, true))
            .collect();
        let mut session = test_session(ships, Vec::new(), true);

        assert!(session.set_tractor_field(0, 1, true));
        assert!(!session.set_tractor_field(0, 1, true));
        assert!(!session.set_tractor_field(0, 2, true));
        assert!(!session.set_tractor_field(2, 1, false));
        assert!(session.set_tractor_field(0, 1, false));
        assert!(session.set_tractor_field(0, 2, true));

        for target in 1..=4 {
            assert!(session.set_gravity_field(0, target, true));
        }
        assert!(!session.set_gravity_field(0, 1, true));
        assert!(!session.set_gravity_field(0, 5, true));
        assert!(!session.set_gravity_field(1, 4, false));
        assert!(session.set_gravity_field(0, 4, false));
        assert!(session.set_gravity_field(0, 5, true));

        let target_two = session
            .field_effects
            .iter()
            .find(|effect| effect.target == 2)
            .expect("target two should retain both field kinds");
        assert_eq!(target_two.source_ids(), (&[0][..], &[0][..]));
        assert_eq!(target_two.resource_id(), Some(3520));
    }

    // Source: FUN_005e45f0 / FUN_005e7540 subsystem condition indicators.
    #[test]
    fn subsystem_condition_uses_exact_five_source_bands() {
        let condition = TacticalSubsystemCondition {
            shields: 0,
            weapons: 24,
            tractor: 25,
            engines: 50,
            hyperdrive: 75,
        };
        assert_eq!(condition.resource_ids(), [1201, 1207, 1213, 1219, 1225]);
        assert_eq!(original_tactical_condition_band(1), 1);
        assert_eq!(original_tactical_condition_band(49), 2);
        assert_eq!(original_tactical_condition_band(74), 3);
        assert_eq!(original_tactical_condition_band(100), 4);
    }

    // Source: FUN_005b0e50 / FUN_005b3f10 subsystem condition from damage.
    #[test]
    fn subsystem_condition_follows_hull_hits_and_hyperdrive_capacity() {
        let condition = TacticalSubsystemCondition::from_source_state(
            980,
            1000,
            TacticalSubsystemCapacity {
                shields: true,
                weapons: true,
                tractor: true,
                engines: true,
                hyperdrive: 2,
            },
            TacticalSubsystemDamage {
                shields: 1,
                weapons: 2,
                tractor: 3,
                engines: 4,
                hyperdrive: 1,
            },
        );
        assert_eq!(condition.percentages(), [73, 48, 23, 0, 50]);
        assert_eq!(condition.resource_ids(), [1204, 1208, 1212, 1216, 1224]);

        let unavailable = TacticalSubsystemCondition::from_source_state(
            100,
            100,
            TacticalSubsystemCapacity {
                shields: false,
                weapons: false,
                tractor: false,
                engines: false,
                hyperdrive: 0,
            },
            TacticalSubsystemDamage::default(),
        );
        assert_eq!(unavailable.percentages(), [0; 5]);
    }

    // Source: FUN_005b1490 repair kind selection in hit order.
    #[test]
    fn subsystem_repair_selection_preserves_source_hit_order() {
        let damage = TacticalSubsystemDamage {
            shields: 1,
            weapons: 1,
            tractor: 2,
            engines: 2,
            hyperdrive: 1,
        };
        assert_eq!(damage.total_hits(), 7);
        let expected = [
            TacticalSubsystemKind::Engines,
            TacticalSubsystemKind::Engines,
            TacticalSubsystemKind::Shields,
            TacticalSubsystemKind::Hyperdrive,
            TacticalSubsystemKind::Tractor,
            TacticalSubsystemKind::Tractor,
            TacticalSubsystemKind::Weapons,
        ];
        for (index, kind) in expected.into_iter().enumerate() {
            assert_eq!(
                original_subsystem_repair_kind(damage, index as u16 + 1),
                Some(kind)
            );
        }
        assert_eq!(original_subsystem_repair_kind(damage, 0), None);
        assert_eq!(original_subsystem_repair_kind(damage, 8), None);
    }

    // Source: FUN_005b1490 repair chance; FUN_005b0330 repair interval.
    #[test]
    fn subsystem_repair_uses_inclusive_damage_control_chance() {
        let mut ship = test_ship(64, 0, true, true);
        ship.subsystem_capacity.engines = true;
        ship.subsystem_damage.engines = 2;
        ship.damage_control = 40;
        ship.refresh_subsystem_condition();
        assert_eq!(attempt_original_subsystem_repair(&mut ship, 0, 0, 1), None);
        assert_eq!(attempt_original_subsystem_repair(&mut ship, 0, 41, 1), None);
        let repair = attempt_original_subsystem_repair(&mut ship, 0, 40, 1)
            .expect("inclusive chance boundary should repair one engine hit");
        assert_eq!(repair.kind, TacticalSubsystemKind::Engines);
        assert_eq!((repair.hits_before, repair.hits_after), (2, 1));
        assert_eq!(ship.subsystem_damage.engines, 1);
        assert_eq!(ship.subsystem_condition.engines, 75);
        assert_eq!(ORIGINAL_SUBSYSTEM_REPAIR_INTERVAL_TICKS, 50 * 4);
    }

    // Source: FUN_005b05c0 sublight conversion; FUN_005ad750 engine mode.
    #[test]
    fn engine_condition_uses_damage_mode_and_active_tractor_power_not_hull() {
        let mut target = test_ship(64, 0, true, true);
        target.hull_current = 10;
        target.hull_max = 100;
        target.subsystem_capacity.engines = true;
        target.sublight_engine_power = 100.0;
        target.maneuver_state_value = Some(4.0);
        target.subsystem_damage.engines = 1;

        let mut source = test_ship(128, 0, false, true);
        source.hull_current = 50;
        source.hull_max = 100;
        source.subsystem_capacity.tractor = true;
        source.tractor_beam_power = 40.0;
        source.subsystem_damage.tractor = 1;

        let mut session = test_session(vec![target, source], Vec::new(), true);
        assert!(session.set_tractor_field(1, 0, true));
        let held = session.subsystem_mobility(0).unwrap();
        assert_eq!(held.base_engine_power, 100.0);
        assert_eq!(held.engine_mode_bonus, 5.0);
        assert_eq!(held.active_tractor_power, 10.0);
        assert_eq!(held.effective_engine_power, 68.75);
        assert_eq!(held.engine_percent, 68);
        assert_eq!(session.ships[0].subsystem_condition.engines, 68);

        assert!(session.set_tractor_field(1, 0, false));
        let released = session.subsystem_mobility(0).unwrap();
        assert_eq!(released.active_tractor_power, 0.0);
        assert_eq!(released.effective_engine_power, 78.75);
        assert_eq!(released.engine_percent, 78);
        assert_eq!(session.ships[0].subsystem_condition.engines, 78);
    }

    // Source: FUN_005ad750 / FUN_005afb70 engine mode bonus clamp.
    #[test]
    fn maneuver_state_produces_the_source_clamped_one_through_nine_bonus() {
        assert_eq!(original_engine_mode_bonus(None), 1.0);
        assert_eq!(original_engine_mode_bonus(Some(0.0)), 9.0);
        assert_eq!(original_engine_mode_bonus(Some(4.0)), 5.0);
        assert_eq!(original_engine_mode_bonus(Some(8.0)), 1.0);
        assert_eq!(original_engine_mode_bonus(Some(9.0)), 1.0);
        assert_eq!(original_engine_mode_bonus(Some(-1.0)), 9.0);
    }

    // Source: FUN_005cd460 turn; FUN_005b2f30 movement and collision.
    #[test]
    fn movement_uses_effective_power_alignment_and_millisecond_integration() {
        let mut alliance = test_ship(64, 0, true, true);
        alliance.subsystem_capacity.engines = true;
        alliance.source_position.z = -56.0;
        alliance.sublight_engine_power = 100.0;
        alliance.maneuver_state_value = Some(4.0);
        alliance.subsystem_damage.engines = 1;

        let mut empire = test_ship(128, 0, false, true);
        empire.subsystem_capacity.engines = true;
        empire.source_position.z = 56.0;
        empire.sublight_engine_power = 100.0;
        empire.maneuver_state_value = Some(4.0);
        empire.subsystem_damage.engines = 1;

        let mut session = test_session(vec![alliance, empire], Vec::new(), true);
        session.advance_original_tactical_movement(250.0);
        assert_eq!(session.ships[0].source_velocity.z, 78.75);
        assert_eq!(session.ships[1].source_velocity.z, -78.75);
        assert_eq!(session.ships[0].source_position.z, -36.3125);
        assert_eq!(session.ships[1].source_position.z, 36.3125);

        session.ships[0].source_desired_forward = TacticalWorldVector {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        session.advance_original_tactical_movement(250.0);
        assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);
        assert_eq!(session.ships[0].source_position.z, -36.3125);

        session.ships[0].source_forward = TacticalWorldVector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        session.ships[0].source_desired_forward = session.ships[0].source_forward;
        session.advance_original_tactical_movement(250.0);
        assert_eq!(session.ships[0].source_velocity.y, 78.75);
        assert_eq!(session.ships[0].source_position.y, 19.6875);
    }

    // Source: FUN_005b2e60 collision volume overlap test.
    #[test]
    fn collision_predicate_preserves_strict_source_boundaries() {
        let envelope = OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 2.0,
        };
        let mut first = test_ship(64, 0, true, true);
        first.source_collision_envelope = Some(envelope);
        let mut second = test_ship(128, 0, false, true);
        second.source_collision_envelope = Some(envelope);

        second.source_position.z = 1.999;
        assert!(original_tactical_objects_overlap(&first, &second));
        second.source_position.z = 2.0;
        assert!(!original_tactical_objects_overlap(&first, &second));
        second.source_position.z = 0.0;
        second.source_position.y = 1.999;
        assert!(original_tactical_objects_overlap(&first, &second));
        second.source_position.y = 2.0;
        assert!(!original_tactical_objects_overlap(&first, &second));
    }

    // Source: FUN_005b2f30 / FUN_005b2e60 collision position restore.
    #[test]
    fn movement_restores_previous_position_when_mesh_envelopes_overlap() {
        let envelope = OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 2.0,
        };
        let mut moving = test_ship(64, 0, true, true);
        moving.subsystem_capacity.engines = true;
        moving.sublight_engine_power = 4.0;
        moving.source_collision_envelope = Some(envelope);
        moving.source_position.z = 0.0;

        let mut fixed = test_ship(128, 0, false, true);
        fixed.source_collision_envelope = Some(envelope);
        fixed.source_position.z = 2.0;

        let mut session = test_session(vec![moving, fixed], Vec::new(), true);
        session.advance_original_tactical_movement(250.0);
        assert_eq!(session.ships[0].source_position.z, 0.0);
        assert_eq!(session.ships[0].source_velocity.z, 5.0);
    }

    // Source: FUN_005ba270 maneuverability; FUN_005cd460 turn rate.
    #[test]
    fn current_to_desired_turn_uses_source_rates_and_normalization() {
        let close = |left: f32, right: f32| assert!((left - right).abs() < 0.000_01);
        close(original_capital_turn_rate(0), std::f32::consts::PI / 12.0);
        close(
            original_capital_turn_rate(4),
            5.0 * std::f32::consts::PI / 18.0,
        );
        close(original_capital_turn_rate(20), std::f32::consts::PI / 2.0);

        let mut ship = test_ship(64, 0, true, true);
        ship.maneuverability = 4;
        ship.source_desired_forward = TacticalWorldVector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        turn_original_tactical_ship(&mut ship, 250.0);
        let turn = 5.0 * std::f32::consts::PI / 72.0;
        close(ship.source_forward.x, turn.sin());
        close(ship.source_forward.y, 0.0);
        close(ship.source_forward.z, turn.cos());
        close(
            ship.source_forward
                .x
                .hypot(ship.source_forward.y)
                .hypot(ship.source_forward.z),
            1.0,
        );

        turn_original_tactical_ship(&mut ship, 10_000.0);
        assert_eq!(ship.source_forward, ship.source_desired_forward);
    }

    // Source: FUN_005a8f70 waypoint arrival; FUN_005cf980 recovery states.
    #[test]
    fn waypoint_arrival_and_fighter_docking_complete_source_executors() {
        let mut carrier = test_ship(64, 0, true, true);
        carrier.subsystem_capacity.engines = true;
        carrier.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        carrier.source_waypoint = Some(TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        carrier.order = TacticalOrder::Anvil;

        let mut fighter = test_fighter(1, true);
        fighter.source_position.z = 3.5;
        fighter.sublight_engine_power = 4.0;
        fighter.order = TacticalOrder::Recover;
        fighter.recovery_state = TacticalFighterRecoveryState::Returning;
        fighter.recovery_target = Some(0);

        let mut session = test_session(vec![carrier], vec![fighter], true);
        session.advance_original_tactical_movement(250.0);
        assert_eq!(
            session.ships[0].source_position,
            TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }
        );
        assert_eq!(session.ships[0].source_waypoint, None);
        assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);
        assert_eq!(session.ships[0].order, TacticalOrder::None);

        session.advance_original_fighter_recovery(250.0);
        assert_eq!(session.fighters[0].source_position.z, 2.5);
        assert_eq!(
            session.fighters[0].recovery_state,
            TacticalFighterRecoveryState::Docking
        );
        assert!(session.fighters[0].alive);

        session.advance_original_fighter_recovery(250.0);
        assert_eq!(
            session.fighters[0].recovery_state,
            TacticalFighterRecoveryState::Recovered
        );
        assert!(!session.fighters[0].alive);
        assert_eq!(session.fighters[0].squad_count, 12);
        assert_eq!(session.fighters[0].order, TacticalOrder::None);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::capital_maneuver_complete(TacticalVoiceFaction::Alliance, 0,),
                TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Alliance, 0,),
            ]
        );
    }

    // Source: FUN_005cf980 / FUN_005cf940 fighter recovery state.
    #[test]
    fn fighter_recovery_warns_each_player_group_once_when_carriers_are_full() {
        let mut carrier = test_ship(64, 0, true, true);
        carrier.fighter_capacity = 1;
        let mut reserved = test_fighter(1, true);
        reserved.recovery_state = TacticalFighterRecoveryState::Returning;
        reserved.recovery_target = Some(0);
        let mut first = test_fighter(1, true);
        first.fleet_squadron_index = 1;
        first.selected = true;
        let mut second = test_fighter(1, true);
        second.fleet_squadron_index = 2;
        second.selected = true;
        let mut enemy = test_fighter(5, false);
        enemy.selected = true;
        let mut session = test_session(vec![carrier], vec![reserved, first, second, enemy], true);

        begin_original_fighter_recovery(&mut session, &[1, 2, 3]);

        assert!(session.fighters[1..].iter().all(|fighter| {
            fighter.recovery_state == TacticalFighterRecoveryState::AwaitingCarrier
        }));
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::fighter_recovery_capacity(
                TacticalVoiceFaction::Alliance,
                0,
            )]
        );
    }

    // Source: FUN_005cf980 fighter recovery completion.
    #[test]
    fn fighter_recovery_completion_waits_for_the_last_live_group_member() {
        let carrier = test_ship(64, 0, true, true);
        let mut docking = test_fighter(1, true);
        docking.recovery_state = TacticalFighterRecoveryState::Docking;
        docking.recovery_target = Some(0);
        let mut still_returning = test_fighter(1, true);
        still_returning.fleet_squadron_index = 1;
        still_returning.recovery_state = TacticalFighterRecoveryState::Returning;
        still_returning.recovery_target = Some(0);
        still_returning.source_position.z = 8.0;
        let mut session = test_session(vec![carrier], vec![docking, still_returning], true);

        session.advance_original_fighter_recovery(250.0);

        assert!(!session.fighters[0].alive);
        assert!(session.fighters[1].alive);
        assert!(session.take_pending_voice_cues().is_empty());

        session.fighters[1].recovery_state = TacticalFighterRecoveryState::Docking;
        session.advance_original_fighter_recovery(250.0);

        assert!(!session.fighters[1].alive);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::fighter_recovery_complete(
                TacticalVoiceFaction::Alliance,
                0,
            )]
        );
    }

    // Source: _DAT_0066d07c / _DAT_0066d094 / _DAT_0066d098 maneuver params.
    #[test]
    fn maneuver_waypoints_preserve_source_rotations_and_scale() {
        let current = TacticalWorldPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let target = TacticalWorldPosition {
            x: 9.0,
            y: 6.0,
            z: 15.0,
        };
        let (sin, cos) = ORIGINAL_MANEUVER_ANGLE.sin_cos();
        let close = |left: f32, right: f32| assert!((left - right).abs() < 0.000_01);

        let left = original_maneuver_waypoint(TacticalOrder::LeftHook, current, target).unwrap();
        close(left.x, 1.0 + (cos * 8.0 - sin * 12.0) * 0.75);
        close(left.y, 5.0);
        close(left.z, 3.0 + (cos * 12.0 + sin * 8.0) * 0.75);

        let right = original_maneuver_waypoint(TacticalOrder::RightHook, current, target).unwrap();
        close(right.x, 1.0 + (cos * 8.0 + sin * 12.0) * 0.75);
        close(right.y, 5.0);
        close(right.z, 3.0 + (cos * 12.0 - sin * 8.0) * 0.75);

        let hammer = original_maneuver_waypoint(TacticalOrder::Hammer, current, target).unwrap();
        close(hammer.x, 7.0);
        close(hammer.y, 2.0 + (cos * 4.0 - sin * 12.0) * 0.75);
        close(hammer.z, 3.0 + (cos * 12.0 + sin * 4.0) * 0.75);

        let anvil = original_maneuver_waypoint(TacticalOrder::Anvil, current, target).unwrap();
        close(anvil.x, 7.0);
        close(anvil.y, 2.0 + (cos * 4.0 + sin * 12.0) * 0.75);
        close(anvil.z, 3.0 + (cos * 12.0 - sin * 4.0) * 0.75);
    }

    // Source: FUN_005a8f70 waypoint; FUN_005cf980 recovery executor.
    #[test]
    fn committed_maneuver_hold_and_recover_enter_original_executors() {
        let mut player = test_ship(64, 0, true, true);
        player.selected = true;
        player.source_position.z = -50.0;
        player.fighter_capacity = 2;
        let mut enemy = test_ship(128, 0, false, true);
        enemy.source_position = TacticalWorldPosition {
            x: 10.0,
            y: 4.0,
            z: 50.0,
        };
        let mut fighter = test_fighter(1, true);
        fighter.selected = true;
        let mut session = test_session(vec![player, enemy], vec![fighter], true);
        session.selected_ship = Some(0);

        assert_eq!(
            assign_selected_command(
                &mut session,
                TacticalOrder::LeftHook,
                Some(TacticalTactic::Surround),
            ),
            (1, 0)
        );
        assert_eq!(session.ships[0].order, TacticalOrder::LeftHook);
        assert_eq!(session.ships[0].tactic, TacticalTactic::Surround);
        assert!(session.ships[0].source_waypoint.is_some());
        assert_ne!(
            session.ships[0].source_desired_forward,
            TacticalWorldVector::ZERO
        );

        assert_eq!(
            assign_selected_command(&mut session, TacticalOrder::HoldPosition, None),
            (1, 0)
        );
        assert_eq!(session.ships[0].source_waypoint, None);
        assert_eq!(
            session.ships[0].source_desired_forward,
            TacticalWorldVector::ZERO
        );
        assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);

        session.ships[0].selected = false;
        session.selected_ship = None;
        session.selected_fighter_group = Some(0);
        assert_eq!(
            assign_selected_command(&mut session, TacticalOrder::Recover, None),
            (0, 1)
        );
        assert_eq!(session.fighters[0].order, TacticalOrder::Recover);
        assert_eq!(
            session.fighters[0].recovery_state,
            TacticalFighterRecoveryState::Returning
        );
        assert_eq!(session.fighters[0].recovery_target, Some(0));
    }

    // Source: FUN_005d0bb0 / FUN_005a8fc0 attack target acquisition.
    #[test]
    fn attack_orders_acquire_first_eligible_typed_target_for_both_unit_classes() {
        let mut player_ship = test_ship(64, 0, true, true);
        player_ship.selected = true;
        let dead_enemy_ship = test_ship(128, 0, false, false);
        let live_enemy_ship = test_ship(129, 1, false, true);

        let mut player_fighter = test_fighter(1, true);
        player_fighter.fighter_group = 0;
        let mut recovered_enemy_fighter = test_fighter(5, false);
        recovered_enemy_fighter.recovery_state = TacticalFighterRecoveryState::Recovered;
        let live_enemy_fighter = test_fighter(6, false);
        let mut session = test_session(
            vec![player_ship, dead_enemy_ship, live_enemy_ship],
            vec![player_fighter, recovered_enemy_fighter, live_enemy_fighter],
            true,
        );
        session.selected_ship = Some(0);

        assert_eq!(
            assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None,),
            (1, 0)
        );
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::CapitalShip(2))
        );

        assert_eq!(
            assign_selected_command(&mut session, TacticalOrder::AttackFighters, None),
            (1, 0)
        );
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::FighterGroup(2))
        );

        session.ships[0].selected = false;
        session.selected_ship = None;
        session.fighters[0].selected = true;
        session.selected_fighter_group = Some(0);
        assert_eq!(
            assign_selected_command(&mut session, TacticalOrder::AttackFighters, None),
            (0, 1)
        );
        assert_eq!(
            session.fighters[0].attack_target,
            Some(TacticalAttackTarget::FighterGroup(2))
        );
    }

    // Source: FUN_005d0bb0 attack target replacement.
    #[test]
    fn attack_executor_preserves_live_engagement_and_replaces_invalid_target() {
        let mut player = test_ship(64, 0, true, true);
        player.selected = true;
        player.attack_target = Some(TacticalAttackTarget::CapitalShip(2));
        let first_enemy = test_ship(128, 0, false, true);
        let second_enemy = test_ship(129, 1, false, true);
        let mut session = test_session(vec![player, first_enemy, second_enemy], Vec::new(), true);
        session.selected_ship = Some(0);

        assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None);
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::CapitalShip(2))
        );

        session.ships[2].alive = false;
        assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None);
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::CapitalShip(1))
        );
    }

    // Source: FUN_005d0bb0 / FUN_005a8fc0 target lifecycle.
    #[test]
    fn attack_target_lifecycle_reacquires_same_class_and_clears_exhausted_lists() {
        let mut player_ship = test_ship(64, 0, true, true);
        player_ship.order = TacticalOrder::AttackCapitalShips;
        player_ship.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut retreating_enemy = test_ship(128, 0, false, true);
        retreating_enemy.retreating = true;
        let live_enemy = test_ship(129, 1, false, true);

        let mut player_fighter = test_fighter(1, true);
        player_fighter.order = TacticalOrder::AttackFighters;
        player_fighter.attack_target = Some(TacticalAttackTarget::FighterGroup(1));
        let mut recovered_enemy_fighter = test_fighter(5, false);
        recovered_enemy_fighter.recovery_state = TacticalFighterRecoveryState::Recovered;
        let live_enemy_fighter = test_fighter(6, false);
        let mut session = test_session(
            vec![player_ship, retreating_enemy, live_enemy],
            vec![player_fighter, recovered_enemy_fighter, live_enemy_fighter],
            true,
        );

        refresh_original_attack_targets(&mut session);
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::CapitalShip(2))
        );
        assert_eq!(
            session.fighters[0].attack_target,
            Some(TacticalAttackTarget::FighterGroup(2))
        );

        session.ships[2].alive = false;
        session.fighters[2].squad_count = 0;
        refresh_original_attack_targets(&mut session);
        assert_eq!(session.ships[0].attack_target, None);
        assert_eq!(session.fighters[0].attack_target, None);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::capital_orders_complete(TacticalVoiceFaction::Alliance, 0,),
                TacticalVoiceCue::fighter_orders_complete(TacticalVoiceFaction::Alliance, 0,),
            ]
        );

        session.ships[0].order = TacticalOrder::AttackFighters;
        refresh_original_attack_targets(&mut session);
        assert_eq!(
            session.ships[0].attack_target, None,
            "an exhausted fighter list must not fall through to a capital target"
        );
        assert!(session.take_pending_voice_cues().is_empty());
    }

    // Source: FUN_005cf980 recovery state reset.
    #[test]
    fn recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio() {
        fn session_with_source_order(order: TacticalOrder) -> BattleSession {
            let mut source = test_ship(64, 0, true, true);
            source.order = order;
            source.attack_target = (order == TacticalOrder::AttackFighters)
                .then_some(TacticalAttackTarget::FighterGroup(0));
            source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
            source.weapon_ranges.laser_cannon = 100.0;

            let mut capital = test_ship(128, 0, false, true);
            capital.hull_current = 1_000;
            capital.hull_max = 1_000;
            capital.shield = 1_000;
            capital.shield_max = 1_000;

            // FUN_005cf980 advances Docking (state 3) to Recovered (state 4)
            // without removing the strategic squadron count. FUN_005d0bb0
            // and FUN_005a8fc0 keep Attack Fighters bound to class 1, so the
            // resulting empty typed list must not fall through to a capital.
            let mut exhausted_fighter = test_fighter(5, false);
            exhausted_fighter.recovery_state = TacticalFighterRecoveryState::Docking;
            exhausted_fighter.squad_count = 12;

            let mut session = test_session(vec![source, capital], vec![exhausted_fighter], true);
            session.paused = false;
            session
        }

        let mut ordered = session_with_source_order(TacticalOrder::AttackFighters);
        let ordered_capital_before = (ordered.ships[1].hull_current, ordered.ships[1].shield);

        ordered.step();

        assert_eq!(
            ordered.fighters[0].recovery_state,
            TacticalFighterRecoveryState::Recovered
        );
        assert!(!ordered.fighters[0].alive);
        assert_eq!(ordered.fighters[0].squad_count, 12);
        assert_eq!(ordered.ships[0].attack_target, None);
        assert_eq!(
            (ordered.ships[1].hull_current, ordered.ships[1].shield),
            ordered_capital_before
        );
        assert!(ordered.weapon_effects.is_empty());
        assert!(ordered.impact_effects.is_empty());
        assert!(ordered.pending_audio_cues.is_empty());

        let mut autonomous = session_with_source_order(TacticalOrder::None);
        let autonomous_capital_before =
            (autonomous.ships[1].hull_current, autonomous.ships[1].shield);

        autonomous.step();

        assert_ne!(
            (autonomous.ships[1].hull_current, autonomous.ships[1].shield),
            autonomous_capital_before
        );
        assert_eq!(autonomous.weapon_effects.len(), 1);
        assert_eq!(autonomous.impact_effects.len(), 1);
        assert!(!autonomous.pending_audio_cues.is_empty());
    }

    // Source: FUN_005d0bb0 / FUN_005a8fc0 explicit attack targeting.
    #[test]
    fn explicit_attack_order_never_redirects_an_invalid_target() {
        let mut source = test_ship(64, 0, true, true);
        source.order = TacticalOrder::AttackCapitalShips;
        source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
        source.weapon_ranges.laser_cannon = 100.0;
        let dead_target = test_ship(128, 0, false, false);
        let live_target = test_ship(129, 1, false, true);
        let mut ships = vec![source, dead_target, live_target];
        let original_hull = ships[2].hull_current;
        let mut fields = Vec::new();
        let mut projectiles = Vec::new();
        let mut impacts = Vec::new();
        let mut audio_cues = Vec::new();
        let mut voice_cues = Vec::new();

        BattleSession::fire_side(
            &mut ships,
            &mut fields,
            &[0],
            &[2],
            &mut projectiles,
            &mut impacts,
            &mut audio_cues,
            &mut voice_cues,
            true,
            TacticalVoiceFaction::Alliance,
            &mut OriginalTacticalRng::new(1),
        );

        assert_eq!(ships[2].hull_current, original_hull);
        assert!(projectiles.is_empty());
        assert!(impacts.is_empty());
        assert!(audio_cues.is_empty());
    }

    // Source: FUN_005b5100 / FUN_005b5b60 / FUN_005b5f50 hull damage selection.
    #[test]
    fn hull_subsystem_selection_preserves_every_source_boundary() {
        assert_eq!(original_hull_subsystem_kind(50.0), None);
        assert_eq!(
            original_hull_subsystem_kind(50.001),
            Some(TacticalSubsystemKind::Shields)
        );
        assert_eq!(
            original_hull_subsystem_kind(65.0),
            Some(TacticalSubsystemKind::Shields)
        );
        assert_eq!(
            original_hull_subsystem_kind(65.001),
            Some(TacticalSubsystemKind::Weapons)
        );
        assert_eq!(
            original_hull_subsystem_kind(80.001),
            Some(TacticalSubsystemKind::Tractor)
        );
        assert_eq!(
            original_hull_subsystem_kind(90.001),
            Some(TacticalSubsystemKind::Engines)
        );
        assert_eq!(
            original_hull_subsystem_kind(95.001),
            Some(TacticalSubsystemKind::Hyperdrive)
        );
        assert_eq!(original_hull_subsystem_kind(100.001), None);
    }

    // Source: FUN_005b54d0 shield damage probability.
    #[test]
    fn shield_damage_uses_inclusive_source_probability() {
        let mut ship = test_ship(64, 0, true, true);
        ship.hull_current = 1000;
        ship.hull_max = 1000;
        ship.shield = 100;
        ship.shield_max = 100;
        ship.subsystem_capacity.shields = true;
        ship.refresh_subsystem_condition();
        let mut ships = vec![ship.clone()];
        let mut fields = Vec::new();
        apply_original_capital_damage(
            &mut ships,
            &mut fields,
            0,
            20,
            OriginalTacticalDamageRolls {
                shield: 20,
                hull: 0,
            },
        );
        assert_eq!(ships[0].subsystem_damage.shields, 1);
        assert_eq!(ships[0].subsystem_condition.shields, 75);

        let mut ships = vec![ship];
        apply_original_capital_damage(
            &mut ships,
            &mut fields,
            0,
            20,
            OriginalTacticalDamageRolls {
                shield: 21,
                hull: 0,
            },
        );
        assert_eq!(ships[0].subsystem_damage.shields, 0);
        assert_eq!(ships[0].subsystem_condition.shields, 100);
    }

    // Source: FUN_005b5100 / FUN_005b5b60 / FUN_005b5f50 hull damage subsystems.
    #[test]
    fn hull_damage_mutates_each_source_subsystem_category() {
        let cases = [
            (
                51,
                TacticalSubsystemDamage {
                    shields: 1,
                    ..TacticalSubsystemDamage::default()
                },
            ),
            (
                66,
                TacticalSubsystemDamage {
                    weapons: 1,
                    ..TacticalSubsystemDamage::default()
                },
            ),
            (
                81,
                TacticalSubsystemDamage {
                    tractor: 1,
                    ..TacticalSubsystemDamage::default()
                },
            ),
            (
                91,
                TacticalSubsystemDamage {
                    engines: 1,
                    ..TacticalSubsystemDamage::default()
                },
            ),
            (
                96,
                TacticalSubsystemDamage {
                    hyperdrive: 1,
                    ..TacticalSubsystemDamage::default()
                },
            ),
        ];
        for (roll, expected) in cases {
            let mut ship = test_ship(64, 0, true, true);
            ship.hull_current = 1000;
            ship.hull_max = 1000;
            ship.subsystem_capacity = TacticalSubsystemCapacity {
                shields: true,
                weapons: true,
                tractor: true,
                engines: true,
                hyperdrive: 2,
            };
            ship.refresh_subsystem_condition();
            let mut ships = vec![ship];
            let stage = apply_original_capital_damage(
                &mut ships,
                &mut Vec::new(),
                0,
                1,
                OriginalTacticalDamageRolls {
                    shield: 100,
                    hull: roll,
                },
            );
            assert_eq!(stage, OriginalTacticalImpactStage::Damage);
            assert_eq!(ships[0].hull_current, 999);
            assert_eq!(ships[0].subsystem_damage, expected);
        }
    }

    // Source: FUN_005b5100 hull damage subsystem destruction path.
    #[test]
    fn destroyed_tractor_subsystem_clears_its_live_field_source() {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 1000;
        source.hull_max = 1000;
        source.subsystem_capacity.tractor = true;
        source.subsystem_damage.tractor = 3;
        source.refresh_subsystem_condition();
        let target = test_ship(128, 0, false, true);
        let mut ships = vec![source, target];
        let mut fields = Vec::new();
        assert!(set_original_tactical_field(
            &mut fields,
            0,
            1,
            OriginalTacticalFieldKind::Tractor,
            true,
        ));

        apply_original_capital_damage(
            &mut ships,
            &mut fields,
            0,
            1,
            OriginalTacticalDamageRolls {
                shield: 100,
                hull: 81,
            },
        );
        assert_eq!(ships[0].subsystem_damage.tractor, 4);
        assert_eq!(ships[0].subsystem_condition.tractor, 0);
        assert!(fields.is_empty());
    }

    // Source: FUN_005b6530 weapon arc classifier.
    #[test]
    fn weapon_arc_classifier_preserves_source_order_and_boundaries() {
        let source = test_ship(64, 0, true, true);
        let target_at = |x: f32, z: f32| {
            let mut target = test_ship(128, 0, false, true);
            target.source_position = TacticalWorldPosition { x, y: 0.0, z };
            target
        };

        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(0.0, 10.0)),
            OriginalTacticalWeaponArc::Fore
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(10.0, 0.0)),
            OriginalTacticalWeaponArc::Starboard
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(0.0, -10.0)),
            OriginalTacticalWeaponArc::Aft
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(-10.0, 0.0)),
            OriginalTacticalWeaponArc::Port
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(10.0, 10.0)),
            OriginalTacticalWeaponArc::Fore,
            "the +45 degree boundary belongs to the fore arc"
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(10.0, -10.0)),
            OriginalTacticalWeaponArc::Starboard,
            "the +135 degree boundary remains lateral"
        );
        assert_eq!(
            original_tactical_weapon_arc(&source, &target_at(-10.0, -10.0)),
            OriginalTacticalWeaponArc::Port,
            "the -135 degree boundary remains lateral"
        );
    }

    // Source: FUN_005b3a40 / FUN_005b3f10 / FUN_005b6530 weapon fire.
    #[test]
    fn direct_weapon_callback_fires_every_family_and_queues_the_arc_once() {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 1000;
        source.hull_max = 1000;
        source.weapon_arcs[0] = TacticalWeaponArc::new(60, 40, 80);
        source.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        };
        source.order = TacticalOrder::AttackCapitalShips;
        source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut target = test_ship(128, 0, false, true);
        target.source_position.z = 10.0;
        target.hull_current = 1000;
        target.hull_max = 1000;
        target.shield = 1000;
        target.shield_max = 1000;
        let mut ships = vec![source, target];
        let mut fields = Vec::new();
        let mut projectiles = Vec::new();
        let mut impacts = Vec::new();
        let mut audio_cues = Vec::new();
        let mut voice_cues = Vec::new();

        BattleSession::fire_side(
            &mut ships,
            &mut fields,
            &[0],
            &[1],
            &mut projectiles,
            &mut impacts,
            &mut audio_cues,
            &mut voice_cues,
            true,
            TacticalVoiceFaction::Alliance,
            &mut OriginalTacticalRng::new(1),
        );

        assert_eq!(
            projectiles
                .iter()
                .map(|effect| effect.kind)
                .collect::<Vec<_>>(),
            vec![
                WeaponKind::IonCannon,
                WeaponKind::LaserCannon,
                WeaponKind::Turbolaser
            ]
        );
        assert_eq!(ships[0].weapon_recharge_queue, vec![0]);
        assert!(!ships[0].weapon_arcs[0].ready);
        assert_eq!(ships[0].weapon_arcs[0].energy, 0.0);
        assert_eq!(ships[1].shield, 965);
        assert_eq!(
            audio_cues.iter().map(|cue| cue.event).collect::<Vec<_>>(),
            vec![
                TacticalAudioEvent::IonFire,
                TacticalAudioEvent::IonImpact,
                TacticalAudioEvent::LaserFire,
                TacticalAudioEvent::LaserImpact,
                TacticalAudioEvent::TurbolaserFire,
                TacticalAudioEvent::TurbolaserImpact,
            ]
        );
        assert!(audio_cues.iter().all(|cue| {
            let base = cue.event.resource_base();
            cue.resource_id >= base && cue.resource_id < base + u32::from(cue.event.variant_count())
        }));
    }

    // Source: FUN_005b3a40 / FUN_005b3f10 automatic weapon fire.
    #[test]
    fn automatic_weapon_callback_keeps_first_arc_on_ties_and_fires_each_ready_arc() {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 100;
        source.hull_max = 100;
        source.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 30);
        source.weapon_arcs[1] = TacticalWeaponArc::new(0, 0, 30);
        source.weapon_ranges.turbolaser = 100.0;
        let mut fore = test_ship(128, 0, false, true);
        fore.source_position.z = 10.0;
        fore.hull_current = 1000;
        fore.hull_max = 1000;
        fore.shield = 1000;
        fore.shield_max = 1000;
        let mut starboard = fore.clone();
        starboard.identity.fleet_roster_index = 1;
        starboard.source_position = TacticalWorldPosition {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        };
        let mut ships = vec![source, fore, starboard];
        let mut projectiles = Vec::new();
        let mut audio_cues = Vec::new();
        let mut voice_cues = Vec::new();

        BattleSession::fire_side(
            &mut ships,
            &mut Vec::new(),
            &[0],
            &[1, 2],
            &mut projectiles,
            &mut Vec::new(),
            &mut audio_cues,
            &mut voice_cues,
            true,
            TacticalVoiceFaction::Alliance,
            &mut OriginalTacticalRng::new(1),
        );

        assert_eq!(
            projectiles
                .iter()
                .map(|effect| effect.target)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(ships[0].weapon_recharge_queue, vec![0, 1]);
        assert_eq!(audio_cues.len(), 4);
    }

    // Source: FUN_005d2360 weapon strength coefficient.
    #[test]
    fn weapon_candidates_enforce_range_and_apply_hull_scaling_only_to_auto_fire() {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 50;
        source.hull_max = 100;
        source.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 100);
        source.weapon_ranges.turbolaser = 10.0;
        let mut target = test_ship(128, 0, false, true);
        target.source_position.z = 10.0;
        let ships = vec![source, target];

        let automatic = original_capital_weapon_candidates(&ships, 0, &[1], None);
        let direct = original_capital_weapon_candidates(&ships, 0, &[1], Some(1));
        assert!((automatic[0].turbolaser.strength - 13.333_334).abs() < 0.000_01);
        assert!((direct[0].turbolaser.strength - 26.666_668).abs() < 0.000_01);

        let mut out_of_range = ships;
        out_of_range[1].source_position.z = 10.001;
        assert_eq!(
            original_capital_weapon_candidates(&out_of_range, 0, &[1], None)[0]
                .turbolaser
                .target,
            None
        );
    }

    // Source: FUN_005b64e0 / FUN_005b6320 weapon recharge FIFO.
    #[test]
    fn weapon_recharge_is_fifo_and_preserves_leftover_energy() {
        let mut ship = test_ship(64, 0, true, true);
        ship.hull_current = 100;
        ship.hull_max = 100;
        ship.weapon_recharge_rate = 8.0;
        ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 10);
        ship.weapon_arcs[1] = TacticalWeaponArc::new(0, 0, 10);
        for arc in &mut ship.weapon_arcs[..2] {
            arc.ready = false;
            arc.energy = 0.0;
        }
        ship.weapon_recharge_queue = vec![0, 1];

        ship.advance_original_recharge(1.0);
        assert_eq!(ship.weapon_arcs[0].energy, 8.0);
        assert_eq!(ship.weapon_arcs[1].energy, 0.0);
        assert_eq!(ship.weapon_recharge_queue, vec![0, 1]);

        ship.advance_original_recharge(0.5);
        assert_eq!(ship.weapon_arcs[0].energy, 10.0);
        assert!(ship.weapon_arcs[0].ready);
        assert_eq!(ship.weapon_arcs[1].energy, 2.0);
        assert_eq!(ship.weapon_recharge_queue, vec![1]);
    }

    // Source: FUN_005b05c0 weapon recharge scale 3.75.
    #[test]
    fn shield_recharge_retains_fractional_source_energy_and_damage_penalties() {
        let mut ship = test_ship(64, 0, true, true);
        ship.hull_current = 100;
        ship.hull_max = 100;
        ship.shield = 0;
        ship.shield_max = 10;
        ship.shield_recharge_rate = 3.0;

        for _ in 0..4 {
            ship.advance_original_recharge(0.25);
        }
        assert_eq!(ship.shield, 3);
        assert_eq!(ship.shield_recharge_carry, 0.0);

        ship.subsystem_damage.shields = 1;
        ship.advance_original_recharge(1.0);
        assert_eq!(ship.shield, 5);
        assert!((ship.shield_recharge_carry - 0.25).abs() < f32::EPSILON);
    }

    // Source: FUN_005b3f10 subsystem condition affecting weapon arcs.
    #[test]
    fn destroyed_weapon_subsystem_disables_and_repair_recharges_live_arcs() {
        let mut ship = test_ship(64, 0, true, true);
        ship.hull_current = 100;
        ship.hull_max = 100;
        ship.subsystem_capacity.weapons = true;
        ship.weapon_recharge_rate = 10.0;
        ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 10);

        for _ in 0..4 {
            ship.add_subsystem_hit(TacticalSubsystemKind::Weapons);
        }
        assert!(!ship.weapon_arcs[0].ready);
        assert_eq!(ship.weapon_arcs[0].energy, 0.0);
        assert_eq!(ship.weapon_recharge_queue, vec![0]);
        ship.remove_subsystem_hit(TacticalSubsystemKind::Weapons);
        ship.advance_original_recharge(4.0);
        assert!(ship.weapon_arcs[0].ready);
        assert_eq!(ship.weapon_arcs[0].energy, 10.0);
        assert!(ship.weapon_recharge_queue.is_empty());
    }

    // Source: FUN_005b3a40 / FUN_005b3f10 / FUN_005b54d0 fire and damage.
    #[test]
    fn live_ship_fire_uses_the_shared_subsystem_damage_path() {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 1000;
        source.hull_max = 1000;
        source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
        source.weapon_ranges.laser_cannon = 100.0;
        let mut target = test_ship(128, 0, false, true);
        target.hull_current = 1000;
        target.hull_max = 1000;
        target.shield = 10;
        target.shield_max = 10;
        target.subsystem_capacity.shields = true;
        target.refresh_subsystem_condition();
        let mut ships = vec![source, target];
        let mut fields = Vec::new();
        let mut projectiles = Vec::new();
        let mut impacts = Vec::new();
        let mut audio_cues = Vec::new();
        let mut voice_cues = Vec::new();

        BattleSession::fire_side(
            &mut ships,
            &mut fields,
            &[0],
            &[1],
            &mut projectiles,
            &mut impacts,
            &mut audio_cues,
            &mut voice_cues,
            true,
            TacticalVoiceFaction::Alliance,
            &mut OriginalTacticalRng::new(1),
        );

        assert!(ships[1].hull_current < 1000);
        assert_eq!(ships[1].subsystem_damage.shields, 1);
        assert!(ships[1].subsystem_condition.shields < 100);
        assert_eq!(projectiles.len(), 1);
        assert_eq!(impacts.len(), 1);
        assert_eq!(audio_cues.len(), 2);
    }

    // Source: FUN_005a7500 / FUN_005bae60 audio event bank.
    #[test]
    fn tactical_audio_event_bank_covers_every_recovered_wave_once() {
        let events = [
            TacticalAudioEvent::LaserFire,
            TacticalAudioEvent::TurbolaserFire,
            TacticalAudioEvent::IonFire,
            TacticalAudioEvent::TorpedoFire,
            TacticalAudioEvent::LaserImpact,
            TacticalAudioEvent::IonImpact,
            TacticalAudioEvent::TurbolaserImpact,
            TacticalAudioEvent::TorpedoImpact,
        ];
        let cues: Vec<_> = events
            .into_iter()
            .flat_map(|event| {
                (0..event.variant_count())
                    .map(move |variant| TacticalAudioCue::from_variant(event, variant))
            })
            .collect();

        assert_eq!(
            cues.iter()
                .map(|cue| cue.event.event_id())
                .collect::<Vec<_>>(),
            vec![
                0x0d, 0x0d, 0x0d, 0x0e, 0x0e, 0x0e, 0x0f, 0x0f, 0x0f, 0x10, 0x10, 0x10, 0x11, 0x11,
                0x11, 0x12, 0x12, 0x12, 0x13, 0x13, 0x13, 0x14,
            ]
        );
        assert_eq!(
            cues.iter().map(|cue| cue.resource_id).collect::<Vec<_>>(),
            (13_033..=13_054).collect::<Vec<_>>()
        );
    }

    // Source: FUN_0061a310 / FUN_005a8a70 MSVC-compatible global RNG.
    #[test]
    fn tactical_rng_preserves_the_recovered_recurrence_and_inclusive_range() {
        let mut rng = OriginalTacticalRng::new(1);
        assert_eq!(
            (0..6).map(|_| rng.next_u15()).collect::<Vec<_>>(),
            vec![41, 18_467, 6_334, 26_500, 19_169, 15_724]
        );

        let mut rng = OriginalTacticalRng::new(1);
        assert_eq!(rng.inclusive(1, 10), 2);
        assert_eq!(rng.inclusive(0, 100), 85);
        assert_ne!(rng.state(), 1);
    }

    #[test]
    fn tactical_voice_cues_preserve_source_event_and_resource_families() {
        // FUN_005bae60 supplies the event/resource arithmetic; the named
        // command callers and audited voice map supply these family offsets.
        assert_eq!(
            TacticalVoiceCue::battle_ready(TacticalVoiceFaction::Alliance),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::BattleReady,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x20,
                resource_id: 14_001,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_attack(TacticalVoiceFaction::Alliance, 3).resource_id,
            14_040
        );
        assert_eq!(
            TacticalVoiceCue::capital_maneuver_complete(TacticalVoiceFaction::Alliance, 7)
                .source_event,
            0x36
        );
        assert_eq!(
            TacticalVoiceCue::fighter_maneuver_complete(TacticalVoiceFaction::Empire, 3)
                .source_event,
            0xb4
        );
        assert_eq!(
            TacticalVoiceCue::capital_orders_complete(TacticalVoiceFaction::Alliance, 7)
                .source_event,
            0x50
        );
        assert_eq!(
            TacticalVoiceCue::fighter_orders_complete(TacticalVoiceFaction::Empire, 3).source_event,
            0xce
        );
        assert_eq!(
            TacticalVoiceCue::target_destroyed_by_capital(TacticalVoiceFaction::Alliance, 7)
                .source_event,
            0x5d
        );
        assert_eq!(
            TacticalVoiceCue::target_destroyed_by_fighter(TacticalVoiceFaction::Empire, 3)
                .source_event,
            0xdb
        );
        assert_eq!(
            TacticalVoiceCue::capital_destroyed(TacticalVoiceFaction::Alliance, 7).source_event,
            0x6a
        );
        assert_eq!(
            TacticalVoiceCue::fighter_losses(TacticalVoiceFaction::Empire, 3).source_event,
            0xe8
        );
        assert_eq!(
            TacticalVoiceCue::capital_formation(TacticalVoiceFaction::Empire, 7),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::CapitalFormation,
                faction: TacticalVoiceFaction::Empire,
                source_event: 0xf4,
                resource_id: 15_091,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_mission(TacticalVoiceFaction::Empire, 3).resource_id,
            15_104
        );
        assert_eq!(
            TacticalVoiceCue::withdrawal_started(TacticalVoiceFaction::Alliance),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::WithdrawalStarted,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x8f,
                resource_id: 14_112,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Alliance, 0),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::FighterRecoveryComplete,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x87,
                resource_id: 14_104,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Empire, 3),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::FighterRecoveryComplete,
                faction: TacticalVoiceFaction::Empire,
                source_event: 0x108,
                resource_id: 15_111,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Alliance, 3),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::FighterRecoveryCapacity,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x8e,
                resource_id: 14_111,
            }
        );
        assert_eq!(
            TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Empire, 0),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::FighterRecoveryCapacity,
                faction: TacticalVoiceFaction::Empire,
                source_event: 0x109,
                resource_id: 15_112,
            }
        );
        assert_eq!(
            TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Alliance),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::WithdrawalHyperdriveWarning,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x95,
                resource_id: 14_118,
            }
        );
        assert_eq!(
            TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Empire),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::WithdrawalHyperdriveWarning,
                faction: TacticalVoiceFaction::Empire,
                source_event: 0x10e,
                resource_id: 15_117,
            }
        );
        assert_eq!(
            TacticalVoiceCue::battle_won(TacticalVoiceFaction::Empire, false).resource_id,
            15_129
        );
        assert_eq!(TacticalVoiceCue::death_star_firing().resource_id, 15_120);
        assert_eq!(TacticalVoiceCue::trench_run_started(0).source_event, 0x139);
        assert_eq!(
            TacticalVoiceCue::trench_run_committed(0).source_event,
            0x13a
        );
        assert_eq!(
            TacticalVoiceCue::trench_run_outcome(3, TacticalTrenchRunOutcome::Success),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::TrenchRunSucceeded,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x138,
                resource_id: 15_159,
            }
        );
        assert_eq!(
            TacticalVoiceCue::target_task_force_rejected(TacticalVoiceFaction::Alliance),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::TargetTaskForceRejected,
                faction: TacticalVoiceFaction::Alliance,
                source_event: 0x84,
                resource_id: 14_101,
            }
        );
        assert_eq!(
            TacticalVoiceCue::target_task_force_rejected(TacticalVoiceFaction::Empire),
            TacticalVoiceCue {
                event: TacticalVoiceEvent::TargetTaskForceRejected,
                faction: TacticalVoiceFaction::Empire,
                source_event: 0x102,
                resource_id: 15_105,
            }
        );

        let mut empire = test_session(Vec::new(), Vec::new(), false);
        empire.queue_battle_ready_voice();
        assert_eq!(
            empire.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_ready(TacticalVoiceFaction::Empire)]
        );
        empire.queue_withdrawal_started_voice();
        assert_eq!(
            empire.take_pending_voice_cues(),
            vec![TacticalVoiceCue::withdrawal_started(
                TacticalVoiceFaction::Empire
            )]
        );
    }

    // Source: FUN_005a5cf0 chooses the local firing group for a destroyed
    // target and the local casualty group when the opponent destroys it.
    #[test]
    fn destruction_voice_callback_uses_source_and_target_group_perspectives() {
        let mut cues = Vec::new();
        queue_original_destruction_voice(
            &mut cues,
            TacticalVoiceFaction::Alliance,
            true,
            true,
            TacticalVoiceGroup::Fighter(2),
            false,
            TacticalVoiceGroup::Capital(4),
            OriginalTacticalImpactStage::Destroyed,
        );
        queue_original_destruction_voice(
            &mut cues,
            TacticalVoiceFaction::Alliance,
            true,
            false,
            TacticalVoiceGroup::Capital(3),
            true,
            TacticalVoiceGroup::Capital(4),
            OriginalTacticalImpactStage::Destroyed,
        );
        queue_original_destruction_voice(
            &mut cues,
            TacticalVoiceFaction::Alliance,
            true,
            false,
            TacticalVoiceGroup::Capital(3),
            true,
            TacticalVoiceGroup::Fighter(1),
            OriginalTacticalImpactStage::Destroyed,
        );
        queue_original_destruction_voice(
            &mut cues,
            TacticalVoiceFaction::Alliance,
            true,
            true,
            TacticalVoiceGroup::Capital(0),
            false,
            TacticalVoiceGroup::Capital(0),
            OriginalTacticalImpactStage::Damage,
        );

        assert_eq!(
            cues,
            vec![
                TacticalVoiceCue::target_destroyed_by_fighter(TacticalVoiceFaction::Alliance, 2,),
                TacticalVoiceCue::capital_destroyed(TacticalVoiceFaction::Alliance, 4),
                TacticalVoiceCue::fighter_losses(TacticalVoiceFaction::Alliance, 1),
            ]
        );
    }

    #[test]
    fn trench_run_groups_route_to_their_exact_source_start_and_outcome_events() {
        // FUN_005bae60 maps these four Alliance group triplets into the
        // recovered 15133..15163 trench-run voice bank.
        let expected = [
            (0, 0x139, 0x13b, 0x13c),
            (1, 0x12d, 0x12f, 0x130),
            (2, 0x131, 0x133, 0x134),
            (3, 0x135, 0x137, 0x138),
        ];

        for (group, start, failure, success) in expected {
            assert_eq!(
                TacticalVoiceCue::trench_run_started(group).source_event,
                start
            );
            assert_eq!(
                TacticalVoiceCue::trench_run_outcome(group, TacticalTrenchRunOutcome::Failure)
                    .source_event,
                failure
            );
            assert_eq!(
                TacticalVoiceCue::trench_run_outcome(group, TacticalTrenchRunOutcome::Success)
                    .source_event,
                success
            );
        }
    }

    #[test]
    fn death_star_attack_entry_routes_each_local_perspective_once() {
        // FUN_005d0340 selects the active Alliance RGBG call for the attacking
        // player and source event 0x115 for the Imperial station owner.
        let mut attacking_fighter = test_fighter(3, true);
        attacking_fighter.fighter_group = 2;
        attacking_fighter.order = TacticalOrder::AttackDeathStar;
        let mut alliance = test_session(Vec::new(), vec![attacking_fighter], true);
        alliance.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        begin_original_attack_order(&mut alliance, &[], &[0], TacticalOrder::AttackDeathStar);
        assert_eq!(
            alliance.take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::trench_run_started(2),
                TacticalVoiceCue::trench_run_committed(2),
                TacticalVoiceCue::trench_run_chatter(0x11e),
            ]
        );
        assert_eq!(
            alliance.trench_run_remaining_ms,
            Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
        );
        assert_eq!(alliance.trench_run_group, Some(2));

        let mut attacking_fighter = test_fighter(3, true);
        attacking_fighter.fighter_group = 2;
        attacking_fighter.order = TacticalOrder::AttackDeathStar;
        let mut empire = test_session(Vec::new(), vec![attacking_fighter], false);
        empire.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        begin_original_attack_order(&mut empire, &[], &[0], TacticalOrder::AttackDeathStar);
        assert_eq!(
            empire.take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::death_star_under_attack(),
                TacticalVoiceCue::trench_run_chatter(0x11e),
            ]
        );
        assert_eq!(
            empire.trench_run_remaining_ms,
            Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
        );
    }

    #[test]
    fn tactical_voice_bank_maps_all_source_events_to_exact_resources() {
        // FUN_005bae60 registers every event from 0x20 through 0x13c.
        let cues = (0x20..=0x13c)
            .map(|source_event| {
                TacticalVoiceCue::from_source_event(TacticalVoiceEvent::SourceMapped, source_event)
            })
            .collect::<Vec<_>>();
        assert_eq!(cues.len(), 285);
        assert_eq!(
            cues.iter()
                .map(|cue| (cue.faction, cue.resource_id))
                .collect::<HashSet<_>>()
                .len(),
            285
        );
        assert_eq!(cues.first().unwrap().resource_id, 14_001);
        assert_eq!(cues[121].resource_id, 14_122);
        assert_eq!(cues[122].resource_id, 15_001);
        assert_eq!(cues[253].resource_id, 15_132);
        assert_eq!(cues[254].resource_id, 15_133);
        assert_eq!(cues.last().unwrap().resource_id, 15_163);
    }

    #[cfg(feature = "interface-test-fixtures")]
    #[test]
    fn tactical_audio_fixture_queues_every_weapon_and_voice_resource_once() {
        // The fixture mirrors FUN_005bae60's complete source-event mapping,
        // grouped by the two owned resource DLLs for deterministic loading.
        let mut state = TacticalState {
            session: Some(test_session(Vec::new(), Vec::new(), true)),
            ..TacticalState::default()
        };

        state.configure_tactical_audio_fixture();

        let session = state.session.as_mut().unwrap();
        assert_eq!(
            session
                .take_pending_audio_cues()
                .into_iter()
                .map(|cue| cue.resource_id)
                .collect::<Vec<_>>(),
            (13_033..=13_054).collect::<Vec<_>>()
        );
        let voice_resources = session
            .take_pending_voice_cues()
            .into_iter()
            .map(|cue| cue.resource_id)
            .collect::<Vec<_>>();
        let expected = (14_001..=14_122)
            .chain(15_133..=15_163)
            .chain(15_001..=15_132)
            .collect::<Vec<_>>();
        assert_eq!(voice_resources, expected);
        assert_eq!(voice_resources.len(), 285);
    }

    // Source: FUN_005d3ac0 field slot sequence.
    #[test]
    fn original_slot_sequence_matches_executable_branch_order() {
        let actual: Vec<_> = OriginalTacticalSlots::default().take(9).collect();
        assert_eq!(actual[0].to_bits(), (-0.0f32).to_bits());
        assert_eq!(
            actual,
            vec![0.0, 5.0, -5.0, -10.0, 10.0, 15.0, -15.0, -20.0, 20.0]
        );
    }

    // Source: FUN_005ab650 / FUN_005a9030 world positions from layout.
    #[test]
    fn production_participants_retain_dat_identity_and_source_world_positions() {
        let mut ships = vec![
            test_ship(64, 0, true, true),
            test_ship(65, 2, true, true),
            test_ship(66, 7, true, false),
            test_ship(133, 0, false, true),
            test_ship(134, 1, false, true),
            test_ship(135, 2, false, true),
        ];
        let mut fighters = vec![test_fighter(3, true), test_fighter(5, false)];

        let layout = BattleSession::assign_original_world_positions(&mut ships, &mut fighters);
        assert_eq!(layout.first_active_objects, 3);
        assert_eq!(layout.second_active_objects, 4);
        assert_eq!(layout.battle_extent, 112.0);
        assert_eq!(
            ships[0].source_position,
            TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: -56.0
            }
        );
        assert_eq!(
            ships[1].source_position,
            TacticalWorldPosition {
                x: 5.0,
                y: 0.0,
                z: -56.0
            }
        );
        assert_eq!(ships[2].source_position, TacticalWorldPosition::ORIGIN);
        assert_eq!(
            ships[3].source_position,
            TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 56.0
            }
        );
        assert_eq!(
            ships[4].source_position,
            TacticalWorldPosition {
                x: 5.0,
                y: 0.0,
                z: 56.0
            }
        );
        assert_eq!(
            ships[5].source_position,
            TacticalWorldPosition {
                x: -5.0,
                y: 0.0,
                z: 56.0
            }
        );
        assert_eq!(
            fighters[0].source_position,
            TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: -36.0
            }
        );
        assert_eq!(
            fighters[1].source_position,
            TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 36.0
            }
        );
        assert_eq!(ships[1].identity.class_dat_id, DatId::new(65));
        assert_eq!(ships[1].identity.fleet_roster_index, 2);
    }

    // Source: FUN_005ae460 fighter group assignment.
    #[test]
    fn fighter_groups_follow_source_ordinals_for_both_factions() {
        let mut fighters = [
            test_fighter(3, true),
            test_fighter(4, true),
            test_fighter(1, true),
            test_fighter(2, true),
            test_fighter(5, false),
            test_fighter(6, false),
            test_fighter(7, false),
            test_fighter(8, false),
        ];
        for fighter in &mut fighters {
            fighter.fighter_group = 4;
        }

        BattleSession::assign_original_fighter_groups(&mut fighters);
        assert_eq!(
            fighters.map(|fighter| fighter.fighter_group),
            [0, 1, 2, 3, 0, 1, 2, 3]
        );
    }

    // Source: FUN_005c81d0 task force leader selection and slots.
    #[test]
    fn task_force_leaders_follow_weapon_categories_square_root_counts_and_source_slots() {
        let mut ships = (0..9)
            .map(|index| test_ship(64, index, true, true))
            .collect::<Vec<_>>();
        for ship in &mut ships[..4] {
            ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 1, 0);
        }
        for ship in &mut ships[4..8] {
            ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
        }

        BattleSession::assign_original_task_forces(&mut ships);

        assert_eq!(
            ships.iter().map(|ship| ship.task_force).collect::<Vec<_>>(),
            vec![0, 0, 1, 1, 2, 2, 3, 3, 4]
        );
        assert_eq!(ships[0].source_position.y, 0.0);
        assert_eq!(ships[2].source_position.y, 8.0);
        assert_eq!(ships[4].source_position.y, -8.0);
        assert_eq!(ships[6].source_position.y, 16.0);
        assert_eq!(ships[8].source_position.y, -16.0);
    }

    // Source: FUN_005c81d0 sorts by -(sublight + 10000 * range).
    #[test]
    fn task_force_capability_order_prioritizes_range_then_available_sublight() {
        let mut ships = (0..4)
            .map(|index| test_ship(64, index, true, true))
            .collect::<Vec<_>>();
        for ship in &mut ships {
            ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
        }
        ships[0].weapon_ranges.laser_cannon = 10.0;
        ships[0].sublight_engine_power = 50.0;
        ships[1].weapon_ranges.laser_cannon = 10.0;
        ships[1].sublight_engine_power = 200.0;
        ships[2].weapon_ranges.laser_cannon = 20.0;
        ships[2].sublight_engine_power = 1.0;
        ships[3].weapon_ranges.laser_cannon = 5.0;
        ships[3].sublight_engine_power = 500.0;

        BattleSession::assign_original_task_forces(&mut ships);

        assert_eq!(
            ships.iter().map(|ship| ship.task_force).collect::<Vec<_>>(),
            vec![1, 0, 0, 1]
        );
        assert_eq!(ships[2].source_position.x, 0.0);
        assert_eq!(ships[0].source_position.x, 0.0);
    }

    // Source: FUN_005ea250 / FUN_005ec400 formation footprint and slots.
    #[test]
    fn task_force_followers_use_largest_footprint_and_alternating_slots() {
        let mut ships = (0..3)
            .map(|index| test_ship(64, index, true, true))
            .collect::<Vec<_>>();
        for ship in &mut ships {
            ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
            ship.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
                vertical_radius: 1.0,
                planar_diameter: 4.0,
            });
        }
        ships[1].source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 2.0,
            planar_diameter: 10.0,
        });
        ships[0].source_position.x = -0.0;

        BattleSession::assign_original_task_forces(&mut ships);

        assert_eq!(ships[0].task_force, 0);
        assert_eq!(ships[1].task_force, 0);
        assert_eq!(ships[2].task_force, 0);
        assert_eq!(ships[0].source_position.x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(ships[1].source_position.x, 12.0);
        assert_eq!(ships[2].source_position.x, -12.0);
        assert!(ships.iter().all(|ship| ship.source_position.y == 0.0));
    }

    // Source: FUN_005ea250 / FUN_005ec400 / FUN_005cbbc0 formation geometry.
    #[test]
    fn task_force_followers_rotate_and_move_with_the_retained_anchor() {
        let mut ships = (0..2)
            .map(|index| test_ship(64, index, true, true))
            .collect::<Vec<_>>();
        for ship in &mut ships {
            ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
            ship.subsystem_capacity.engines = true;
            ship.sublight_engine_power = 4.0;
            ship.source_forward = TacticalWorldVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            };
            ship.source_desired_forward = ship.source_forward;
            ship.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
                vertical_radius: 1.0,
                planar_diameter: 10.0,
            });
        }
        BattleSession::assign_original_task_forces(&mut ships);
        let mut session = test_session(ships, Vec::new(), true);

        session.advance_original_tactical_movement(250.0);

        assert_eq!(session.ships[0].source_position.x, 1.25);
        assert_eq!(session.ships[1].source_position.x, 1.25);
        assert_eq!(session.ships[0].source_position.z, 0.0);
        assert_eq!(session.ships[1].source_position.z, -12.0);
        assert_eq!(
            session.ships[0].source_position.y,
            session.ships[1].source_position.y
        );
    }

    // Source: FUN_005ae460 fighter group split threshold.
    #[test]
    fn empty_fighter_group_splits_first_half_only_above_seven_members() {
        let mut ten = (0..10)
            .map(|index| {
                let mut fighter = test_fighter(3, true);
                fighter.fleet_squadron_index = index;
                fighter.fighter_group = 4;
                fighter
            })
            .collect::<Vec<_>>();
        BattleSession::assign_original_fighter_groups(&mut ten);
        assert!(ten[..5].iter().all(|fighter| fighter.fighter_group == 1));
        assert!(ten[5..].iter().all(|fighter| fighter.fighter_group == 0));

        let mut seven = (0..7)
            .map(|_| {
                let mut fighter = test_fighter(3, true);
                fighter.fighter_group = 4;
                fighter
            })
            .collect::<Vec<_>>();
        BattleSession::assign_original_fighter_groups(&mut seven);
        assert!(seven.iter().all(|fighter| fighter.fighter_group == 0));
    }

    // Source: FUN_005b05c0 / FUN_005b9c60 sublight conversion for fighters.
    #[test]
    fn fighter_roster_expands_each_strategic_squadron_with_exact_dat_stats() {
        let mut world = GameWorld::default();
        let class = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                dat_id: DatId::new(1),
                name: "X-wing".into(),
                is_alliance: true,
                squadron_size: 12,
                shield_strength: 5,
                sub_light_engine: 11,
                maneuverability: 8,
                laser_cannon_fore: 5,
                laser_cannon_range: 12,
                torpedoes: 4,
                torpedoes_range: 7,
                ..rebellion_core::world::FighterClass::default()
            });
        let fleet = world.fleets.insert(rebellion_core::world::Fleet {
            location: SystemKey::default(),
            capital_ships: Vec::new(),
            fighters: vec![rebellion_core::world::FighterEntry { class, count: 2 }],
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
        });
        let mut ships = Vec::new();
        let mut fighters = Vec::new();

        BattleSession::expand_fleet(&world, fleet, true, &mut ships, &mut fighters);

        assert!(ships.is_empty());
        assert_eq!(fighters.len(), 2);
        assert_eq!(fighters[0].fleet_squadron_index, 0);
        assert_eq!(fighters[1].fleet_squadron_index, 1);
        assert_eq!(fighters[0].squad_count, 12);
        assert_eq!(fighters[0].hull_current, 96.0);
        assert_eq!(fighters[0].shield, 5.0);
        assert_eq!(fighters[0].sublight_engine_power, 4.4);
        assert_eq!(fighters[0].weapon_arc.laser_cannons, 5);
        assert_eq!(fighters[0].weapon_ranges.laser_cannon, 12.0);
        assert_eq!(fighters[0].torpedo_strength, 4);
        assert_eq!(fighters[0].torpedo_range, 7.0);
    }

    // Source: FUN_005b9c00 / FUN_005b9c30 fighter hull scaling.
    #[test]
    fn fighter_hull_drives_weapon_shield_recharge_and_live_craft_count() {
        let mut fighter = test_fighter(1, true);
        assert!((fighter.weapon_hull_factor() - 45.012).abs() < 0.001);
        assert!((fighter.shield_recharge_rate() - 0.3).abs() < 0.001);

        fighter.weapon_arc.ready = false;
        fighter.weapon_arc.energy = 0.0;
        fighter.shield = 0.0;
        fighter.advance_original_recharge(1.0);
        assert!(fighter.weapon_arc.ready);
        assert_eq!(
            fighter.weapon_arc.energy,
            fighter.weapon_arc.energy_capacity
        );
        assert!((fighter.shield - 0.3).abs() < 0.001);

        fighter.hull_current = 79.9;
        fighter.refresh_craft_count();
        assert_eq!(fighter.squad_count, 10);
        fighter.hull_current = 0.0;
        fighter.refresh_craft_count();
        assert_eq!(fighter.squad_count, 0);
        assert!(!fighter.alive);
    }

    // Source: FUN_005d2490 / FUN_005d24e0 fighter damage modifiers.
    #[test]
    fn fighter_damage_uses_float_shields_hull_and_maneuver_ratio() {
        let mut target = test_fighter(5, false);
        target.shield = 5.0;
        target.shield_max = 5.0;
        target.maneuverability = 7;
        let mut fighters = vec![target];

        assert_eq!(
            apply_original_fighter_damage(&mut fighters, 0, 9.0, 4.0),
            OriginalTacticalImpactStage::Hit
        );
        assert!((fighters[0].shield - 0.5).abs() < 0.001);
        assert_eq!(fighters[0].hull_current, 96.0);

        assert_eq!(
            apply_original_fighter_damage(&mut fighters, 0, 9.0, 4.0),
            OriginalTacticalImpactStage::Damage
        );
        assert_eq!(fighters[0].shield, 0.0);
        assert!((fighters[0].hull_current - 92.0).abs() < 0.001);
        assert_eq!(fighters[0].squad_count, 12);
    }

    // Source: FUN_005b23e0 through FUN_005b25d0 fighter battery sources.
    #[test]
    fn fighter_fore_battery_fires_source_families_and_torpedoes_at_capitals() {
        let mut fighter = test_fighter(2, true);
        fighter.weapon_arc = TacticalWeaponArc::new(8, 6, 0);
        fighter.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: 20.0,
            ion_cannon: 18.0,
            turbolaser: 0.0,
        };
        fighter.torpedo_strength = 12;
        fighter.torpedo_range = 7.0;
        fighter.order = TacticalOrder::AttackCapitalShips;
        fighter.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        fighter.source_position.z = 0.0;
        let mut target = test_ship(128, 0, false, true);
        target.source_position.z = 6.0;
        target.hull_current = 1000;
        target.hull_max = 1000;
        target.shield = 0;
        target.shield_max = 0;
        let mut session = test_session(vec![target], vec![fighter], true);

        session.fighter_step();

        assert!(session.ships[0].hull_current < 1000);
        assert!(!session.fighters[0].weapon_arc.ready);
        assert_eq!(session.weapon_effects.len(), 3);
        assert_eq!(session.weapon_effects[0].kind, WeaponKind::IonCannon);
        assert_eq!(session.weapon_effects[1].kind, WeaponKind::LaserCannon);
        assert_eq!(session.weapon_effects[2].kind, WeaponKind::FighterAttack);
        assert_eq!(session.weapon_effects[0].source, usize::MAX);
        assert_eq!(
            session.weapon_effects[0].target_position,
            session.ships[0].source_position
        );
        assert_eq!(
            session
                .pending_audio_cues
                .iter()
                .map(|cue| cue.event)
                .collect::<Vec<_>>(),
            vec![
                TacticalAudioEvent::IonFire,
                TacticalAudioEvent::IonImpact,
                TacticalAudioEvent::LaserFire,
                TacticalAudioEvent::LaserImpact,
                TacticalAudioEvent::TorpedoFire,
                TacticalAudioEvent::TorpedoImpact,
            ]
        );
    }

    // Source: FUN_005d2490 / FUN_005d24e0 fighter-vs-fighter modifiers.
    #[test]
    fn dogfight_ignores_ion_and_applies_target_maneuverability() {
        let mut source = test_fighter(2, true);
        source.weapon_arc = TacticalWeaponArc::new(8, 6, 0);
        source.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: 20.0,
            ion_cannon: 18.0,
            turbolaser: 0.0,
        };
        source.maneuverability = 2;
        source.order = TacticalOrder::AttackFighters;
        source.attack_target = Some(TacticalAttackTarget::FighterGroup(1));
        let mut target = test_fighter(6, false);
        target.shield = 0.0;
        target.shield_max = 0.0;
        target.maneuverability = 8;
        target.weapon_arc = TacticalWeaponArc::default();
        target.source_position.z = 10.0;
        let mut session = test_session(Vec::new(), vec![source, target], true);

        session.fighter_step();

        assert!(session.fighters[1].hull_current < 96.0);
        assert!(session.fighters[1].hull_current > 60.0);
        assert_eq!(session.weapon_effects.len(), 1);
        assert_eq!(session.weapon_effects[0].kind, WeaponKind::LaserCannon);
        assert_eq!(session.weapon_effects[0].target, usize::MAX);
    }

    // Source: FUN_005d0bb0 / FUN_005a8fc0 typed attack target.
    #[test]
    fn capital_attack_fighters_uses_typed_target_without_capital_fallback() {
        let mut source = test_ship(64, 0, true, true);
        source.weapon_arcs[0] = TacticalWeaponArc::new(300, 10, 0);
        source.weapon_ranges.laser_cannon = 100.0;
        source.weapon_ranges.ion_cannon = 100.0;
        source.maneuverability = 8;
        source.order = TacticalOrder::AttackFighters;
        source.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        let mut target = test_fighter(5, false);
        target.shield = 0.0;
        target.shield_max = 0.0;
        target.source_position.z = 10.0;
        let mut session = test_session(vec![source], vec![target], true);
        let mut effects = Vec::new();

        session.fire_capitals_at_fighters(&mut effects);

        assert!(!session.fighters[0].alive);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].kind, WeaponKind::LaserCannon);
        assert!(!session.ships[0].weapon_arcs[0].ready);
        assert_eq!(
            session
                .pending_audio_cues
                .iter()
                .map(|cue| cue.event)
                .collect::<Vec<_>>(),
            vec![
                TacticalAudioEvent::LaserFire,
                TacticalAudioEvent::LaserImpact
            ]
        );
    }

    #[test]
    fn original_tactical_canvas_letterboxes_without_distorting_controls() {
        let native = TacticalCanvas::new(640.0, 480.0);
        assert_eq!((native.x, native.y, native.scale), (0.0, 0.0, 1.0));
        let wide = TacticalCanvas::new(1280.0, 800.0);
        assert_eq!(wide.scale, 5.0 / 3.0);
        assert_eq!(wide.x, (1280.0 - 640.0 * wide.scale) * 0.5);
        let (screen_x, screen_y) = wide.point(574.0, 317.0);
        let (logical_x, logical_y) = wide.logical_pointer(screen_x, screen_y);
        assert!((logical_x - 574.0).abs() < 0.001);
        assert!((logical_y - 317.0).abs() < 0.001);
    }

    // Source: FUN_005f0f40 control rect boundaries.
    #[test]
    fn original_tactical_control_rects_stop_at_bitmap_edges() {
        assert_eq!(
            tactical_hud_rect_control_at(560.0, 307.0),
            Some(TacticalHudControl::Pause)
        );
        assert_eq!(
            tactical_hud_rect_control_at(587.9, 327.9),
            Some(TacticalHudControl::Pause)
        );
        assert_eq!(tactical_hud_rect_control_at(588.0, 317.0), None);
        assert_eq!(tactical_hud_rect_control_at(559.9, 317.0), None);
        assert_eq!(
            tactical_hud_rect_control_at(486.0, 343.0),
            Some(TacticalHudControl::ZoomIn)
        );
        assert_eq!(tactical_hud_rect_control_at(510.0, 355.0), None);
        assert_eq!(
            tactical_hud_rect_control_at(603.0, 343.0),
            Some(TacticalHudControl::ZoomOut)
        );
        assert_eq!(
            tactical_hud_rect_control_at(517.0, 304.0),
            Some(TacticalHudControl::HighlightAlliance)
        );
        assert_eq!(
            tactical_hud_rect_control_at(482.0, 304.0),
            Some(TacticalHudControl::HighlightEmpire)
        );
        assert_eq!(
            tactical_hud_rect_control_at(485.0, 272.0),
            Some(TacticalHudControl::NavigationSet(0))
        );
        assert_eq!(
            tactical_hud_rect_control_at(627.9, 298.9),
            Some(TacticalHudControl::NavigationSet(3))
        );
        assert_eq!(tactical_hud_rect_control_at(628.0, 285.0), None);
        assert_eq!(
            tactical_hud_rect_control_at(484.0, 430.0),
            Some(TacticalHudControl::CameraRecall)
        );
        assert_eq!(
            tactical_hud_rect_control_at(627.9, 454.9),
            Some(TacticalHudControl::CameraMemorize)
        );
        assert_eq!(tactical_hud_rect_control_at(628.0, 442.0), None);
    }

    // Source: FUN_005f0f40 / FUN_005f1050 bitmap hit masks.
    #[test]
    #[ignore = "requires extracted original bitmaps in data/base/ui"]
    fn original_tactical_controls_use_source_bitmap_hit_masks() {
        let mut cache = BmpCache::new();
        cache.set_base_path(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui"),
        );

        // Original control hit testing excludes every outer edge, even when
        // the containing rectangle starts there.
        assert_eq!(tactical_hud_control_at(&mut cache, 560.0, 307.0), None);
        assert_eq!(
            tactical_hud_control_at(&mut cache, 562.1, 308.1),
            Some(TacticalHudControl::Pause)
        );

        // Both zoom controls contain transparent matte pixels inside their
        // bounding rectangles. Those pixels must pass through untouched.
        assert_eq!(tactical_hud_control_at(&mut cache, 487.1, 344.1), None);
        assert_eq!(
            tactical_hud_control_at(&mut cache, 491.1, 344.1),
            Some(TacticalHudControl::ZoomIn)
        );
        assert_eq!(tactical_hud_control_at(&mut cache, 604.1, 344.1), None);
        assert_eq!(
            tactical_hud_control_at(&mut cache, 609.1, 344.1),
            Some(TacticalHudControl::ZoomOut)
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 520.1, 397.1),
            Some(TacticalHudControl::CameraLeft)
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 590.1, 397.1),
            Some(TacticalHudControl::CameraRight)
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 558.1, 354.1),
            Some(TacticalHudControl::CameraUp)
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 558.1, 444.1),
            Some(TacticalHudControl::CameraDown)
        );

        assert_eq!(
            tactical_hud_control_at(&mut cache, 518.1, 305.1),
            Some(TacticalHudControl::HighlightAlliance)
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 483.1, 305.1),
            Some(TacticalHudControl::HighlightEmpire)
        );
        assert_eq!(tactical_hud_control_at(&mut cache, 588.0, 317.0), None);
        assert_eq!(
            tactical_hud_control_at(&mut cache, 616.0, 318.0),
            Some(TacticalHudControl::BattleOptions)
        );

        assert_eq!(
            tactical_battle_options_control_at(&mut cache, true, 521.0, 64.0),
            Some(TacticalBattleOptionsControl::Withdraw)
        );
        assert_eq!(
            tactical_battle_options_control_at(&mut cache, false, 521.0, 64.0),
            None
        );
        assert_eq!(
            tactical_battle_options_control_at(&mut cache, true, 519.0, 126.0),
            Some(TacticalBattleOptionsControl::SimulateRemainder)
        );
        assert_eq!(
            tactical_battle_options_control_at(&mut cache, true, 583.0, 126.0),
            Some(TacticalBattleOptionsControl::ObserveToggle)
        );
        assert_eq!(
            tactical_battle_options_control_at(&mut cache, true, 520.0, 219.0),
            Some(TacticalBattleOptionsControl::GameOptions)
        );
        assert_eq!(
            tactical_battle_options_control_at(&mut cache, true, 579.0, 219.0),
            Some(TacticalBattleOptionsControl::Close)
        );
        assert_eq!(
            tactical_withdraw_confirmation_control_at(&mut cache, 560.0, 242.0),
            Some(TacticalWithdrawConfirmationControl::Confirm)
        );
        assert_eq!(
            tactical_withdraw_confirmation_control_at(&mut cache, 600.0, 242.0),
            Some(TacticalWithdrawConfirmationControl::Cancel)
        );
        assert_eq!(
            tactical_withdraw_confirmation_control_at(&mut cache, 547.0, 229.0),
            None
        );
        assert_eq!(
            tactical_withdraw_confirmation_control_at(&mut cache, 614.0, 242.0),
            None
        );

        assert_eq!(
            tactical_hud_control_at(&mut cache, 72.1, 10.1),
            Some(TacticalHudControl::TaskForce(0))
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 254.1, 10.1),
            Some(TacticalHudControl::TaskForce(7))
        );
        assert_eq!(tactical_hud_control_at(&mut cache, 60.1, 2.1), None);
        assert_eq!(
            tactical_hud_control_at(&mut cache, 342.1, 10.1),
            Some(TacticalHudControl::FighterGroup(0))
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 420.1, 10.1),
            Some(TacticalHudControl::FighterGroup(3))
        );
    }

    // Source: FUN_005f0c20 builds TACTICAL 1310, FUN_005f0f40 places confirm and
    // cancel, and FUN_005f1170 dispatches the withdrawal on confirm (P58-B13).
    #[test]
    fn withdrawal_requires_native_confirmation_before_dispatch() {
        let mut state = TacticalState {
            session: Some(test_session(Vec::new(), Vec::new(), true)),
            ..TacticalState::default()
        };
        state.battle_options_open = true;

        assert_eq!(
            activate_tactical_battle_options(&mut state, TacticalBattleOptionsControl::Withdraw,),
            TacticalAction::None
        );
        assert!(!state.battle_options_open);
        assert!(state.withdraw_confirmation_open);

        assert_eq!(
            activate_tactical_withdraw_confirmation(
                &mut state,
                TacticalWithdrawConfirmationControl::Cancel,
            ),
            TacticalAction::None
        );
        assert!(!state.withdraw_confirmation_open);
        assert!(state
            .session
            .as_ref()
            .unwrap()
            .pending_voice_cues
            .is_empty());

        state.battle_options_open = true;
        assert_eq!(
            activate_tactical_battle_options(&mut state, TacticalBattleOptionsControl::Withdraw,),
            TacticalAction::None
        );
        assert_eq!(
            activate_tactical_withdraw_confirmation(
                &mut state,
                TacticalWithdrawConfirmationControl::Confirm,
            ),
            TacticalAction::WithdrawFromBattle
        );
        assert!(!state.withdraw_confirmation_open);
        assert_eq!(
            state.session.as_mut().unwrap().take_pending_voice_cues(),
            vec![TacticalVoiceCue::withdrawal_started(
                TacticalVoiceFaction::Alliance
            )]
        );

        let mut operational = test_ship(64, 0, false, true);
        operational.is_attacker = false;
        operational.subsystem_capacity.hyperdrive = 1;
        operational.refresh_subsystem_condition();
        let mut stranded = test_ship(64, 1, false, true);
        stranded.is_attacker = false;
        stranded.subsystem_capacity.hyperdrive = 1;
        stranded.subsystem_damage.hyperdrive = 1;
        stranded.refresh_subsystem_condition();
        let mut empire_state = TacticalState {
            session: Some(test_session(vec![operational, stranded], Vec::new(), false)),
            withdraw_confirmation_open: true,
            ..TacticalState::default()
        };
        assert_eq!(
            activate_tactical_withdraw_confirmation(
                &mut empire_state,
                TacticalWithdrawConfirmationControl::Confirm,
            ),
            TacticalAction::WithdrawFromBattle
        );
        assert_eq!(
            empire_state
                .session
                .as_mut()
                .unwrap()
                .take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::withdrawal_started(TacticalVoiceFaction::Empire),
                TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Empire),
            ]
        );
    }

    // Source: REBDLG.DLL resource IDs 10710-10973 battle alert frames.
    #[test]
    fn battle_alert_skins_match_recovered_faction_resources_and_geometry() {
        let alliance = battle_alert_skin(true);
        assert_eq!(alliance.frame, 10_710);
        assert_eq!(alliance.scene, 10_712);
        assert_eq!(alliance.rail, 10_820);
        assert_eq!(alliance.tabs[0], [10_728, 10_729]);
        assert_eq!(alliance.tabs[3], [10_734, 10_735]);
        assert_eq!(alliance.buttons[0], [10_971, 10_972, 10_973]);
        assert_eq!(alliance.buttons[2], [10_719, 10_720, 10_721]);
        assert_eq!(
            alliance.tab_rects,
            [
                NativeRect::new(418.0, 21.0, 41.0, 41.0),
                NativeRect::new(418.0, 81.0, 41.0, 41.0),
                NativeRect::new(418.0, 143.0, 41.0, 41.0),
                NativeRect::new(418.0, 205.0, 41.0, 41.0),
            ]
        );

        let empire = battle_alert_skin(false);
        assert_eq!(empire.frame, 10_711);
        assert_eq!(empire.scene, 10_713);
        assert_eq!(empire.rail, 10_821);
        assert_eq!(empire.tabs[0], [10_738, 10_739]);
        assert_eq!(empire.tabs[3], [10_744, 10_745]);
        assert_eq!(empire.buttons[0], [10_974, 10_975, 10_976]);
        assert_eq!(empire.buttons[2], [10_725, 10_726, 10_727]);
        assert_eq!(
            empire.tab_rects,
            [
                NativeRect::new(426.0, 17.0, 44.0, 41.0),
                NativeRect::new(426.0, 80.0, 44.0, 41.0),
                NativeRect::new(426.0, 143.0, 44.0, 41.0),
                NativeRect::new(426.0, 206.0, 44.0, 41.0),
            ]
        );
    }

    // Source: manual pages 143 and 148, task-force and fighter-group selection (P58E).
    #[test]
    fn task_force_and_fighter_group_selection_are_mutually_exclusive() {
        let mut ships = (0..8)
            .map(|index| test_ship(64, index, true, true))
            .collect::<Vec<_>>();
        ships[0].selected = true;
        let mut fighters = (0..4)
            .map(|index| {
                let mut fighter = test_fighter(1, true);
                fighter.fighter_group = index;
                fighter.identity.fleet_roster_index = usize::from(index);
                fighter.fleet_fighter_index = usize::from(index);
                fighter
            })
            .collect::<Vec<_>>();
        fighters.push(test_fighter(5, false));
        let mut session = test_session(ships, fighters, true);
        session.selected_ship = Some(0);

        assert_eq!(select_task_force(&mut session, 7), 1);
        assert_eq!(selected_player_task_force(&session), Some(7));
        assert_eq!(session.selected_ship, Some(7));

        assert!(select_fighter_group(&mut session, 3));
        assert_eq!(session.selected_ship, None);
        assert_eq!(session.selected_fighter_group, Some(3));
        assert!(session.ships.iter().all(|ship| !ship.selected));
        assert_eq!(
            session
                .fighters
                .iter()
                .filter(|fighter| fighter.selected)
                .count(),
            1
        );

        assert_eq!(select_task_force(&mut session, 0), 1);
        assert_eq!(session.selected_fighter_group, None);
        assert!(session.fighters.iter().all(|fighter| !fighter.selected));
    }

    // Source: manual pages 143 and 148, the Ctrl task-force assignment rule (P58E).
    #[test]
    fn task_force_assignment_accepts_only_an_existing_or_next_blank_slot() {
        let mut ships = vec![
            test_ship(64, 0, true, true),
            test_ship(65, 1, true, true),
            test_ship(128, 0, false, true),
        ];
        ships[0].selected = true;
        ships[1].selected = true;
        let mut session = test_session(ships, Vec::new(), true);

        assert_eq!(assign_selected_to_task_force(&mut session, 4), 0);
        assert_eq!(session.ships[0].task_force, 0);
        assert_eq!(session.ships[1].task_force, 1);
        assert_eq!(assign_selected_to_task_force(&mut session, 2), 2);
        assert_eq!(session.ships[0].task_force, 2);
        assert_eq!(session.ships[1].task_force, 2);
    }

    #[test]
    fn selected_capital_navigation_wraps_within_the_current_task_force() {
        let mut ships = vec![
            test_ship(64, 0, true, true),
            test_ship(65, 1, true, true),
            test_ship(66, 2, true, true),
            test_ship(128, 0, false, true),
        ];
        for ship in &mut ships[..3] {
            ship.task_force = 2;
            ship.selected = true;
        }
        let mut session = test_session(ships, vec![test_fighter(1, true)], true);
        session.selected_ship = Some(0);

        assert_eq!(cycle_selected_capital(&mut session, -1), Some((0, 2, 2)));
        assert_eq!(session.selected_ship, Some(2));
        assert_eq!(
            session
                .ships
                .iter()
                .enumerate()
                .filter_map(|(index, ship)| ship.selected.then_some(index))
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(cycle_selected_capital(&mut session, 1), Some((2, 0, 2)));
        assert_eq!(cycle_selected_capital(&mut session, 1), Some((0, 1, 2)));
        assert!(session.fighters.iter().all(|fighter| !fighter.selected));
    }

    // Source: FUN_005cf190 / FUN_005cf8f0 complete a reached waypoint and clear its
    // movement intent (P58F11).
    #[test]
    fn ordered_navigation_route_advances_without_losing_the_next_waypoint() {
        let first = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let second = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        };
        let mut ship = test_ship(64, 0, true, true);
        ship.subsystem_capacity.engines = true;
        ship.source_waypoint = Some(first);
        ship.navigation_route = vec![first, second];
        let mut session = test_session(vec![ship], Vec::new(), true);

        session.advance_original_tactical_movement(10.0);
        assert_eq!(session.ships[0].source_position, first);
        assert_eq!(session.ships[0].source_waypoint, Some(second));
        assert_eq!(session.ships[0].navigation_route, vec![second]);

        session.advance_original_tactical_movement(10.0);
        assert_eq!(session.ships[0].source_position, second);
        assert_eq!(session.ships[0].source_waypoint, None);
        assert!(session.ships[0].navigation_route.is_empty());
    }

    // Source: FUN_005d0b10 / FUN_005d0bb0 manual target list.
    #[test]
    fn ordered_manual_targets_advance_and_clear_as_hostiles_are_destroyed() {
        let mut player = test_ship(64, 0, true, true);
        player.manual_targets = vec![
            TacticalAttackTarget::CapitalShip(1),
            TacticalAttackTarget::CapitalShip(2),
        ];
        player.attack_target = player.manual_targets.first().copied();
        let mut session = test_session(
            vec![
                player,
                test_ship(128, 0, false, true),
                test_ship(129, 1, false, true),
            ],
            Vec::new(),
            true,
        );

        session.ships[1].alive = false;
        refresh_original_attack_targets(&mut session);
        assert_eq!(
            session.ships[0].manual_targets,
            vec![TacticalAttackTarget::CapitalShip(2)]
        );
        assert_eq!(
            session.ships[0].attack_target,
            Some(TacticalAttackTarget::CapitalShip(2))
        );

        session.ships[2].alive = false;
        refresh_original_attack_targets(&mut session);
        assert!(session.ships[0].manual_targets.is_empty());
        assert_eq!(session.ships[0].attack_target, None);
    }

    #[test]
    fn alliance_mixed_task_forces_reject_focus_target_without_mutating_orders() {
        // FUN_005a24d0 queues source event 0x84 when the selected Alliance
        // capitals cannot resolve to one task-force ordinal.
        let mut first = test_ship(64, 0, true, true);
        first.selected = true;
        first.task_force = 1;
        first.manual_targets = vec![TacticalAttackTarget::CapitalShip(3)];
        first.attack_target = first.manual_targets.first().copied();
        first.escort_target = Some(3);
        let mut second = test_ship(65, 1, true, true);
        second.selected = true;
        second.task_force = 2;
        second.manual_targets = vec![TacticalAttackTarget::CapitalShip(4)];
        second.attack_target = second.manual_targets.first().copied();
        second.escort_target = Some(4);
        let hostile = test_ship(128, 2, false, true);
        let mut session = test_session(vec![first, second, hostile], Vec::new(), true);
        let before = session.ships[..2]
            .iter()
            .map(|ship| {
                (
                    ship.manual_targets.clone(),
                    ship.attack_target,
                    ship.escort_target,
                )
            })
            .collect::<Vec<_>>();

        let result = assign_selected_capital_focus_target(&mut session, 2, false);

        assert_eq!(
            result,
            TacticalFocusAssignment::RejectedMixedTaskForces { member_count: 2 }
        );
        assert_eq!(
            session.ships[..2]
                .iter()
                .map(|ship| (
                    ship.manual_targets.clone(),
                    ship.attack_target,
                    ship.escort_target
                ))
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::target_task_force_rejected(
                TacticalVoiceFaction::Alliance
            )]
        );
    }

    #[test]
    fn imperial_mixed_task_forces_reject_focus_target_without_mutating_orders() {
        // FUN_005a24d0 queues source event 0x102 for the same rejected command
        // when the player-side tactical manager is Imperial.
        let hostile = test_ship(64, 0, true, true);
        let mut first = test_ship(128, 1, false, true);
        first.selected = true;
        first.task_force = 3;
        first.manual_targets = vec![TacticalAttackTarget::CapitalShip(0)];
        first.attack_target = first.manual_targets.first().copied();
        let mut second = test_ship(129, 2, false, true);
        second.selected = true;
        second.task_force = 4;
        second.escort_target = Some(0);
        let mut session = test_session(vec![hostile, first, second], Vec::new(), false);
        let before = session.ships[1..]
            .iter()
            .map(|ship| {
                (
                    ship.manual_targets.clone(),
                    ship.attack_target,
                    ship.escort_target,
                )
            })
            .collect::<Vec<_>>();

        let result = assign_selected_capital_focus_target(&mut session, 0, true);

        assert_eq!(
            result,
            TacticalFocusAssignment::RejectedMixedTaskForces { member_count: 2 }
        );
        assert_eq!(
            session.ships[1..]
                .iter()
                .map(|ship| (
                    ship.manual_targets.clone(),
                    ship.attack_target,
                    ship.escort_target
                ))
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::target_task_force_rejected(
                TacticalVoiceFaction::Empire
            )]
        );
    }

    // Source: FUN_005a24d0 focus target constraint.
    #[test]
    fn one_task_force_assigns_focus_target_without_rejection_voice() {
        let mut first = test_ship(64, 0, true, true);
        first.selected = true;
        first.task_force = 5;
        let mut second = test_ship(65, 1, true, true);
        second.selected = true;
        second.task_force = 5;
        let hostile = test_ship(128, 2, false, true);
        let mut session = test_session(vec![first, second, hostile], Vec::new(), true);

        let result = assign_selected_capital_focus_target(&mut session, 2, false);

        assert_eq!(
            result,
            TacticalFocusAssignment::Assigned {
                object_ids: vec![1, 2],
                maximum_targets: 1,
            }
        );
        assert!(session.ships[..2].iter().all(|ship| {
            ship.manual_targets == vec![TacticalAttackTarget::CapitalShip(2)]
                && ship.attack_target == Some(TacticalAttackTarget::CapitalShip(2))
        }));
        assert!(session.take_pending_voice_cues().is_empty());
    }

    // Source: FUN_005a24d0 focus target toggle.
    #[test]
    fn appending_an_existing_focus_target_toggles_it_off() {
        let mut friendly = test_ship(64, 0, true, true);
        friendly.selected = true;
        friendly.task_force = 5;
        friendly.manual_targets = vec![TacticalAttackTarget::CapitalShip(1)];
        friendly.attack_target = friendly.manual_targets.first().copied();
        let hostile = test_ship(128, 1, false, true);
        let mut session = test_session(vec![friendly, hostile], Vec::new(), true);

        let result = assign_selected_capital_focus_target(&mut session, 1, true);

        assert_eq!(
            result,
            TacticalFocusAssignment::Assigned {
                object_ids: Vec::new(),
                maximum_targets: 0,
            }
        );
        assert!(session.ships[0].manual_targets.is_empty());
        assert_eq!(session.ships[0].attack_target, None);
        assert!(session.take_pending_voice_cues().is_empty());
    }

    #[test]
    fn focus_target_with_no_selected_capitals_is_a_quiet_noop() {
        let friendly = test_ship(64, 0, true, true);
        let hostile = test_ship(128, 1, false, true);
        let mut session = test_session(vec![friendly, hostile], Vec::new(), true);
        let before = session.ships[0].manual_targets.clone();

        let result = assign_selected_capital_focus_target(&mut session, 1, false);

        assert_eq!(
            result,
            TacticalFocusAssignment::Assigned {
                object_ids: Vec::new(),
                maximum_targets: 0,
            }
        );
        assert_eq!(session.ships[0].manual_targets, before);
        assert!(session.take_pending_voice_cues().is_empty());
    }

    // Source: FUN_005ca6d0 dispatches order codes 0-11 (tactical order
    // execution evidence, 2026-09-17); the tactic toggle at +0x38 uses Surround
    // 1 and Stand Off 2 (tactical command assignment evidence, 2026-09-17).
    #[test]
    fn tactical_order_and_tactic_codes_match_the_recovered_source_domain() {
        let orders = [
            (TacticalOrder::None, 0),
            (TacticalOrder::Escort, 1),
            (TacticalOrder::Recover, 2),
            (TacticalOrder::AttackFighters, 4),
            (TacticalOrder::AttackCapitalShips, 5),
            (TacticalOrder::AttackDeathStar, 6),
            (TacticalOrder::LeftHook, 7),
            (TacticalOrder::RightHook, 8),
            (TacticalOrder::Hammer, 9),
            (TacticalOrder::Anvil, 10),
            (TacticalOrder::HoldPosition, 11),
        ];
        for (order, code) in orders {
            assert_eq!(order.source_code(), code);
        }
        assert_eq!(TacticalTactic::Surround.source_code(), 1);
        assert_eq!(TacticalTactic::StandOff.source_code(), 2);
    }

    // Source: FUN_005ca6d0 dispatches Escort as order code 1; FUN_005d0af0,
    // FUN_005cf910 and FUN_005cf920 install, retain and clear its target (P58-B14).
    #[test]
    fn direct_escort_retains_friendly_target_and_clears_when_it_is_destroyed() {
        let mut escort = test_ship(64, 0, true, true);
        escort.selected = true;
        let target = test_ship(65, 1, true, true);
        let enemy = test_ship(128, 0, false, true);
        let mut session = test_session(vec![escort, target, enemy], Vec::new(), true);
        session.selected_ship = Some(0);

        assert_eq!(assign_selected_escort(&mut session, 1), (1, 0));
        assert_eq!(session.ships[0].order, TacticalOrder::Escort);
        assert_eq!(session.ships[0].escort_target, Some(1));
        advance_original_escort_movement(&mut session, ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        assert!(session.ships[0].source_waypoint.is_some());

        session.ships[1].alive = false;
        refresh_original_escort_targets(&mut session);
        assert_eq!(session.ships[0].order, TacticalOrder::None);
        assert_eq!(session.ships[0].escort_target, None);
        assert_eq!(session.ships[0].source_waypoint, None);
    }

    // Source: FUN_0040bcd0 counts the system's usable family 0x25 facilities
    // (FUN_0051c270) and FUN_005af4e0 stores the answer at +0x70;
    // FUN_005ba450 then refuses the fighters' attack (manual p. 124).
    #[test]
    fn a_death_star_shield_on_the_battle_system_bars_the_fighters_attack() {
        use rebellion_core::world::{DefenseFacilityInstance, Fleet, System};
        let mut world = GameWorld::default();
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Endor".into(),
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
            fighters: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: rebellion_core::world::ControlKind::Controlled(
                rebellion_core::dat::Faction::Empire,
            ),
        });
        let mut fleet = |is_alliance: bool, has_death_star: bool| {
            world.fleets.insert(Fleet {
                location: system,
                capital_ships: vec![],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star,
            })
        };
        let rebels = fleet(true, false);
        let empire = fleet(false, true);
        let battery = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2300_0002),
            side: rebellion_core::dat::Faction::Empire,
        });
        world.systems[system].defense_facilities.push(battery);

        let open = BattleSession::new(&world, system, rebels, empire, true, 1, 1);
        let death_star = open.death_star.expect("the Empire's Death Star");
        assert!(!death_star.shielded);
        assert!(death_star.attackable());

        // DEFFACSD 4, the Death Star shield (TEXTSTRA 8832).
        let shield = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2500_0004),
            side: rebellion_core::dat::Faction::Empire,
        });
        world.systems[system].defense_facilities.push(shield);

        let mut shielded = BattleSession::new(&world, system, rebels, empire, true, 1, 1);
        let death_star = shielded.death_star.expect("the Empire's Death Star");
        assert!(death_star.shielded);
        assert!(death_star.operational());
        assert!(!death_star.attackable());

        // The mission control (FUN_005ad930) needs a selected fighter group
        // and an enemy Death Star the fighters may attack.
        // One Alliance group on the field (FUN_005ad930 counts them).
        let mut open = open;
        open.fighters.push(test_fighter(1, true));
        shielded.fighters.push(test_fighter(1, true));
        assert!(!attack_death_star_enabled(&open));
        open.selected_fighter_group = Some(0);
        assert!(attack_death_star_enabled(&open));
        shielded.selected_fighter_group = Some(0);
        assert!(!attack_death_star_enabled(&shielded));
        let mut own = BattleSession::new(&world, system, rebels, empire, false, 1, 1);
        own.fighters.push(test_fighter(1, true));
        own.selected_fighter_group = Some(0);
        own.death_star.as_mut().unwrap().shielded = false;
        assert!(!attack_death_star_enabled(&own));
    }

    // Source: FUN_005ad930. The manual (p. 151): more Alliance than
    // Imperial fighters launched, and one group at a time.
    #[test]
    fn attack_death_star_needs_more_alliance_fighters_and_no_run_under_way() {
        let mut session = test_session(
            Vec::new(),
            vec![test_fighter(1, true), test_fighter(2, false)],
            true,
        );
        session.selected_fighter_group = Some(0);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        // One each: not more Alliance than Imperial.
        assert!(!attack_death_star_enabled(&session));
        session.fighters.push(test_fighter(3, true));
        assert!(attack_death_star_enabled(&session));
        // A lost Alliance group no longer counts.
        session.fighters[2].alive = false;
        assert!(!attack_death_star_enabled(&session));
        session.fighters[2].alive = true;
        // A run under way.
        session.trench_run_remaining_ms = Some(1_000.0);
        assert!(!attack_death_star_enabled(&session));
        session.trench_run_remaining_ms = None;
        // One of the player's groups already on the order.
        session.fighters[0].order = TacticalOrder::AttackDeathStar;
        assert!(!attack_death_star_enabled(&session));
        // An enemy group's order does not count.
        session.fighters[0].order = TacticalOrder::None;
        session.fighters[1].order = TacticalOrder::AttackDeathStar;
        assert!(attack_death_star_enabled(&session));
    }

    // Source: FUN_005ba7f0 Death Star laser charge rate to 100.
    #[test]
    fn death_star_operational_predicate_resources_and_laser_charge_match_source_fields() {
        let mut death_star = TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        };
        assert!(death_star.operational());
        assert_eq!(death_star.resource_id(), 5010);
        death_star.advance_laser_charge(300.0);
        assert_eq!(death_star.laser_charge, 100.0);
        death_star.advance_laser_charge(1.0);
        assert_eq!(death_star.laser_charge, 100.0);

        death_star.action_committed = true;
        assert!(!death_star.operational());
        death_star.action_committed = false;
        death_star.destroyed = true;
        assert!(!death_star.operational());
        assert_eq!(death_star.resource_id(), 5020);
    }

    #[test]
    fn death_star_ready_voice_fires_once_only_when_the_players_laser_crosses_full_charge() {
        // FUN_005bae60 maps source event 0x113 to the Imperial ready line;
        // the tactical manager emits it on the player's not-ready/ready edge.
        let attacker = test_ship(64, 0, true, true);
        let defender = test_ship(128, 0, false, true);
        let mut session = test_session(vec![attacker, defender], Vec::new(), true);
        session.paused = false;
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: true,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 99.95,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });

        assert!(!session.step());
        assert_eq!(session.combat_tick, 1);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::death_star_ready()]
        );
        assert!(!session.step());
        assert_eq!(session.combat_tick, 2);
        assert!(session.take_pending_voice_cues().is_empty());

        let attacker = test_ship(64, 0, true, true);
        let defender = test_ship(128, 0, false, true);
        let mut opponent_owned = test_session(vec![attacker, defender], Vec::new(), true);
        opponent_owned.paused = false;
        opponent_owned.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 99.95,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });

        assert!(!opponent_owned.step());
        assert_eq!(opponent_owned.combat_tick, 1);
        assert!(opponent_owned.take_pending_voice_cues().is_empty());
    }

    #[test]
    fn battle_outcome_voice_distinguishes_draw_destruction_and_each_withdrawal_side() {
        // FUN_005bae60 assigns separate outcome events for withdrawal,
        // destruction victory, and defeat in each faction's recovered bank.
        let alliance = test_ship(64, 0, true, true);
        let empire = test_ship(128, 0, false, true);
        let mut session = test_session(vec![alliance, empire], Vec::new(), true);

        session.queue_battle_outcome_voice(CombatWinner::Draw);
        assert!(session.take_pending_voice_cues().is_empty());

        session.queue_battle_outcome_voice(CombatWinner::Attacker);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_won(
                TacticalVoiceFaction::Alliance,
                false
            )]
        );

        session.ships[1].retreated = true;
        session.queue_battle_outcome_voice(CombatWinner::Attacker);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_won(
                TacticalVoiceFaction::Alliance,
                true
            )]
        );

        session.ships[1].retreated = false;
        session.ships[0].retreated = true;
        session.queue_battle_outcome_voice(CombatWinner::Defender);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::withdrawal_complete(
                TacticalVoiceFaction::Alliance
            )]
        );

        session.ships[0].retreated = false;
        session.queue_battle_outcome_voice(CombatWinner::Defender);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_lost(
                TacticalVoiceFaction::Alliance
            )]
        );

        let alliance = test_ship(64, 0, true, true);
        let empire = test_ship(128, 0, false, true);
        let mut imperial_player = test_session(vec![alliance, empire], Vec::new(), false);
        imperial_player.ships[0].retreated = true;
        imperial_player.queue_battle_outcome_voice(CombatWinner::Defender);
        assert_eq!(
            imperial_player.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_won(
                TacticalVoiceFaction::Empire,
                true
            )]
        );
        imperial_player.ships[0].retreated = false;
        imperial_player.queue_battle_outcome_voice(CombatWinner::Attacker);
        assert_eq!(
            imperial_player.take_pending_voice_cues(),
            vec![TacticalVoiceCue::battle_lost(TacticalVoiceFaction::Empire)]
        );
    }

    // Source: FUN_005ba7f0 Death Star superlaser predicates.
    #[test]
    fn death_star_superlaser_requires_player_ownership_full_charge_and_enemy_target() {
        let target = test_ship(64, 0, true, true);
        let mut session = test_session(vec![target], Vec::new(), false);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 99.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        assert!(!session.commit_death_star_shot(0));
        session.death_star.as_mut().unwrap().laser_charge = 100.0;
        session.player_is_attacker = true;
        assert!(!session.commit_death_star_shot(0));
        session.player_is_attacker = false;
        session.ships[0].is_attacker = false;
        assert!(!session.commit_death_star_shot(0));
    }

    // Source: FUN_005ba7f0 Death Star superlaser resolution timing.
    #[test]
    fn death_star_superlaser_resolves_after_the_source_long_projectile_duration() {
        let mut target = test_ship(64, 0, true, true);
        target.source_position.z = -24.0;
        let mut session = test_session(vec![target], Vec::new(), false);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 24.0,
            },
            hull: 100.0,
            laser_charge: 100.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });

        assert!(session.commit_death_star_shot(0));
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::death_star_firing()]
        );
        assert_eq!(session.death_star.unwrap().laser_charge, 0.0);
        assert!(session.death_star.unwrap().action_committed);
        assert!(session.death_star_beam.is_some());
        session.advance_presentational_effects(1.999);
        assert!(session.ships[0].alive);
        session.advance_presentational_effects(0.002);
        assert!(!session.ships[0].alive);
        assert_eq!(session.ships[0].hull_current, 0);
        assert!(session.death_star_beam.is_none());
        assert!(!session.death_star.unwrap().action_committed);
        assert_eq!(session.impact_effects.len(), 1);
    }

    // Source: FUN_005cfec0 / FUN_005d04e0 timed trench-run producer.
    #[test]
    fn attack_death_star_order_uses_the_timed_native_result_producer() {
        let mut fighter = test_fighter(3, true);
        fighter.selected = true;
        fighter.order = TacticalOrder::AttackDeathStar;
        fighter.maneuverability = 9;
        let mut session = test_session(Vec::new(), vec![fighter], true);
        session.trench_run_success_rating = 9;
        session.tactical_rng = OriginalTacticalRng::new(15);
        session.selected_fighter_group = Some(0);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 6.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });

        begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
        assert_eq!(
            session.trench_run_remaining_ms,
            Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
        );
        session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS - 1.0);
        assert_eq!(session.trench_run_outcome, None);
        assert_eq!(session.death_star.unwrap().hull, 100.0);
        session.advance_original_trench_run(1.0);

        let death_star = session.death_star.unwrap();
        assert!(death_star.destroyed);
        assert_eq!(death_star.hull, 0.0);
        assert_eq!(death_star.resource_id(), 5020);
        assert!(session.weapon_effects.is_empty());
        assert_eq!(
            session.trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Success)
        );
        assert_eq!(
            session.take_pending_trench_run_cinematic(),
            Some(TacticalTrenchRunOutcome::Success)
        );
        assert_eq!(session.take_pending_trench_run_cinematic(), None);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![
                TacticalVoiceCue::trench_run_started(0),
                TacticalVoiceCue::trench_run_committed(0),
                TacticalVoiceCue::trench_run_chatter(0x11e),
                TacticalVoiceCue::trench_run_chatter(0x11f),
                TacticalVoiceCue::trench_run_chatter(0x120),
                TacticalVoiceCue::trench_run_chatter(0x128),
                TacticalVoiceCue::trench_run_chatter(0x121),
                TacticalVoiceCue::trench_run_chatter(0x122),
                TacticalVoiceCue::trench_run_chatter(0x129),
                TacticalVoiceCue::trench_run_outcome(0, TacticalTrenchRunOutcome::Success),
            ]
        );
    }

    // Source: FUN_005cfec0 state 6 removes the first bounded half of the
    // retained participant list after a successful trench run.
    #[test]
    fn successful_trench_run_applies_the_source_half_group_casualty_rule() {
        let mut fighters = (0..4)
            .map(|_| {
                let mut fighter = test_fighter(3, true);
                fighter.order = TacticalOrder::AttackDeathStar;
                fighter.maneuverability = 9;
                fighter
            })
            .collect::<Vec<_>>();
        fighters[0].selected = true;
        let mut session = test_session(Vec::new(), fighters, true);
        session.trench_run_success_rating = 9;
        session.tactical_rng = OriginalTacticalRng::new(15);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });

        begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
        assert_eq!(session.trench_run_participants, vec![0, 1, 2, 3]);
        assert_eq!(
            session.trench_run_pending_outcome,
            Some(TacticalTrenchRunOutcome::Success)
        );
        session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);

        assert!(!session.fighters[0].alive);
        assert!(!session.fighters[1].alive);
        assert!(session.fighters[2].alive);
        assert!(session.fighters[3].alive);
        assert!(session.death_star.unwrap().destroyed);
    }

    // Source: FUN_005bae60 Death Star attack failure voice event.
    #[test]
    fn losing_the_committed_death_star_attackers_routes_failure_once() {
        let mut fighter = test_fighter(3, true);
        fighter.order = TacticalOrder::AttackDeathStar;
        fighter.weapon_arc = TacticalWeaponArc::default();
        fighter.shield = 0.0;
        fighter.shield_max = 0.0;
        fighter.hull_current = 1.0;
        fighter.refresh_craft_count();

        let mut defender = test_ship(128, 0, false, true);
        defender.order = TacticalOrder::AttackFighters;
        defender.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        defender.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
        defender.weapon_ranges.laser_cannon = 100.0;

        let mut session = test_session(vec![defender], vec![fighter], true);
        session.paused = false;
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 6.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);

        assert!(session.step());
        assert!(!session.fighters[0].alive);
        assert!(!session.death_star.unwrap().destroyed);
        assert_eq!(
            session.trench_run_pending_outcome,
            Some(TacticalTrenchRunOutcome::Failure)
        );
        assert_eq!(session.take_pending_trench_run_cinematic(), None);
        session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
        assert_eq!(
            session.take_pending_trench_run_cinematic(),
            Some(TacticalTrenchRunOutcome::Failure)
        );
        assert_eq!(session.take_pending_trench_run_cinematic(), None);
        let cues = session.take_pending_voice_cues();
        assert_eq!(cues.first(), Some(&TacticalVoiceCue::trench_run_started(0)));
        assert!(cues.contains(&TacticalVoiceCue::fighter_losses(
            TacticalVoiceFaction::Alliance,
            0,
        )));
        assert!(cues.contains(&TacticalVoiceCue::battle_lost(
            TacticalVoiceFaction::Alliance,
        )));
        assert_eq!(
            cues.last(),
            Some(&TacticalVoiceCue::trench_run_outcome(
                0,
                TacticalTrenchRunOutcome::Failure,
            ))
        );
    }

    #[test]
    fn trench_run_result_is_recorded_once_and_failure_remains_source_driven() {
        let mut session = test_session(Vec::new(), Vec::new(), true);
        session.record_trench_run_outcome(TacticalTrenchRunOutcome::Failure);
        session.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);

        assert!(session.paused);
        assert_eq!(
            session.take_pending_trench_run_cinematic(),
            Some(TacticalTrenchRunOutcome::Failure)
        );
        assert_eq!(
            session.trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Failure)
        );
        assert_eq!(session.take_pending_trench_run_cinematic(), None);
        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::trench_run_outcome(
                0,
                TacticalTrenchRunOutcome::Failure
            )]
        );
    }

    #[test]
    fn alliance_trench_result_selects_only_the_active_alliance_attack_group() {
        // FUN_005bae60's RGBY outcome family is selected from the Alliance
        // group actively executing the Death Star mission.
        let mut imperial_attacker = test_fighter(5, false);
        imperial_attacker.fighter_group = 1;
        imperial_attacker.order = TacticalOrder::AttackDeathStar;
        let mut inactive_alliance = test_fighter(3, true);
        inactive_alliance.fighter_group = 2;
        inactive_alliance.order = TacticalOrder::None;
        let mut active_alliance = test_fighter(4, true);
        active_alliance.fighter_group = 3;
        active_alliance.order = TacticalOrder::AttackDeathStar;
        let mut session = test_session(
            Vec::new(),
            vec![imperial_attacker, inactive_alliance, active_alliance],
            true,
        );

        session.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);

        assert_eq!(
            session.take_pending_voice_cues(),
            vec![TacticalVoiceCue::trench_run_outcome(
                3,
                TacticalTrenchRunOutcome::Success
            )]
        );
    }

    #[test]
    fn imperial_trench_run_observer_routes_failure_and_success_to_death_star_events() {
        // FUN_005bae60 supplies distinct Imperial events for a broken-off
        // Alliance attack and destruction of the Death Star.
        let mut failure = test_session(Vec::new(), Vec::new(), false);
        failure.record_trench_run_outcome(TacticalTrenchRunOutcome::Failure);
        assert_eq!(
            failure.take_pending_voice_cues(),
            vec![TacticalVoiceCue::death_star_attack_broken_off()]
        );
        assert_eq!(
            failure.trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Failure)
        );

        let mut success = test_session(Vec::new(), Vec::new(), false);
        success.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);
        assert_eq!(
            success.take_pending_voice_cues(),
            vec![TacticalVoiceCue::death_star_destroyed()]
        );
        assert_eq!(
            success.trench_run_outcome,
            Some(TacticalTrenchRunOutcome::Success)
        );
    }

    #[test]
    fn tactical_assignment_hit_regions_follow_the_source_panels() {
        assert_eq!(
            tactical_command_rect_control_at(
                TacticalCommandPanel::Display,
                true,
                false,
                false,
                493.0,
                60.0,
            ),
            Some(TacticalCommandControl::PreviousCapital)
        );
        assert_eq!(
            tactical_command_rect_control_at(
                TacticalCommandPanel::Display,
                true,
                false,
                false,
                618.0,
                60.0,
            ),
            Some(TacticalCommandControl::NextCapital)
        );
        assert_eq!(
            tactical_command_rect_control_at(
                TacticalCommandPanel::Display,
                true,
                true,
                false,
                493.0,
                60.0,
            ),
            None
        );
        assert_eq!(
            tactical_command_rect_control_at(
                TacticalCommandPanel::Display,
                true,
                false,
                false,
                500.0,
                245.0,
            ),
            Some(TacticalCommandControl::OpenMissions)
        );
        assert_eq!(
            tactical_command_rect_control_at(
                TacticalCommandPanel::Display,
                true,
                false,
                false,
                570.0,
                245.0,
            ),
            Some(TacticalCommandControl::OpenManeuvers)
        );
        let maneuvers = TacticalCommandPanel::Maneuvers {
            pending_order: TacticalOrder::None,
            pending_tactic: TacticalTactic::StandOff,
        };
        assert_eq!(
            tactical_command_rect_control_at(maneuvers, true, false, false, 500.0, 160.0),
            Some(TacticalCommandControl::Hammer)
        );
        assert_eq!(
            tactical_command_rect_control_at(maneuvers, true, false, false, 600.0, 170.0),
            Some(TacticalCommandControl::ToggleTactic)
        );
        let missions = TacticalCommandPanel::Missions {
            pending_order: TacticalOrder::None,
        };
        assert_eq!(
            tactical_command_rect_control_at(missions, true, false, false, 575.0, 165.0),
            None
        );
        assert_eq!(
            tactical_command_rect_control_at(missions, true, true, false, 575.0, 165.0),
            Some(TacticalCommandControl::Recover)
        );
        assert_eq!(
            tactical_command_rect_control_at(missions, true, true, false, 575.0, 195.0),
            None
        );
        assert_eq!(
            tactical_command_rect_control_at(missions, true, true, true, 575.0, 195.0),
            Some(TacticalCommandControl::AttackDeathStar)
        );
        assert_eq!(
            tactical_command_rect_control_at(missions, false, true, false, 505.0, 165.0),
            None
        );
    }

    // Source: FUN_005dcc70's maneuver panel commits order +0x68 and tactic +0x38 on
    // check and returns unchanged on cancel (P58F9).
    #[test]
    fn maneuver_confirm_commits_and_cancel_preserves_selected_capital_orders() {
        let mut ship = test_ship(64, 0, true, true);
        ship.selected = true;
        let mut enemy = test_ship(128, 0, false, true);
        enemy.source_position.z = 50.0;
        let mut session = test_session(vec![ship, enemy], Vec::new(), true);
        session.selected_ship = Some(0);
        let mut state = TacticalState {
            session: Some(session),
            ..TacticalState::default()
        };

        activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
        activate_tactical_command(&mut state, TacticalCommandControl::Hammer);
        activate_tactical_command(&mut state, TacticalCommandControl::ToggleTactic);
        activate_tactical_command(&mut state, TacticalCommandControl::Cancel);
        let ship = &state.session.as_ref().unwrap().ships[0];
        assert_eq!(ship.order, TacticalOrder::None);
        assert_eq!(ship.tactic, TacticalTactic::StandOff);
        assert!(state
            .session
            .as_ref()
            .unwrap()
            .pending_voice_cues
            .is_empty());

        activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
        activate_tactical_command(&mut state, TacticalCommandControl::Hammer);
        activate_tactical_command(&mut state, TacticalCommandControl::ToggleTactic);
        activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
        let ship = &state.session.as_ref().unwrap().ships[0];
        assert_eq!(ship.order, TacticalOrder::Hammer);
        assert_eq!(ship.tactic, TacticalTactic::Surround);
        assert_eq!(state.command_panel, TacticalCommandPanel::Display);
        assert_eq!(
            state.session.as_ref().unwrap().pending_voice_cues,
            vec![TacticalVoiceCue::capital_formation(
                TacticalVoiceFaction::Alliance,
                0
            )]
        );
    }

    // Source: FUN_005def00's mission panel enables Recover (order code 2) for a
    // selected fighter group and commits it to every member (P58F9).
    #[test]
    fn mission_confirm_assigns_recover_to_the_whole_selected_fighter_group() {
        let mut fighters = vec![test_fighter(1, true), test_fighter(1, true)];
        for (index, fighter) in fighters.iter_mut().enumerate() {
            fighter.selected = true;
            fighter.identity.fleet_roster_index = index;
            fighter.fleet_fighter_index = index;
        }
        let mut session = test_session(Vec::new(), fighters, true);
        session.selected_fighter_group = Some(0);
        let mut state = TacticalState {
            session: Some(session),
            ..TacticalState::default()
        };

        activate_tactical_command(&mut state, TacticalCommandControl::OpenMissions);
        activate_tactical_command(&mut state, TacticalCommandControl::Recover);
        activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
        assert!(state
            .session
            .as_ref()
            .unwrap()
            .fighters
            .iter()
            .all(|fighter| fighter.order == TacticalOrder::Recover));
        assert_eq!(
            state.session.as_ref().unwrap().pending_voice_cues,
            vec![
                TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Alliance, 0,),
                TacticalVoiceCue::fighter_mission(TacticalVoiceFaction::Alliance, 0),
            ]
        );
    }

    // Source: TACTICAL.DLL resources 1058/1059 paint the 23 by 23 target control
    // at (538,379) after the D-pad (tactical target control evidence,
    // 2026-09-13). Centering on the 1200 by 800 arena is the port's 2D fallback.
    #[test]
    #[ignore = "requires extracted original bitmaps in data/base/ui"]
    fn target_control_owns_its_topmost_mask_and_centers_the_selected_position() {
        let mut cache = BmpCache::new();
        cache.set_base_path(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui"),
        );
        assert_eq!(
            tactical_hud_control_at(&mut cache, 549.1, 390.1),
            Some(TacticalHudControl::CameraTarget)
        );
        assert_eq!(camera_offset_for_target(120.0, 190.0), (-480.0, -210.0));
        assert_eq!(camera_offset_for_target(600.0, 400.0), (0.0, 0.0));
    }
}
