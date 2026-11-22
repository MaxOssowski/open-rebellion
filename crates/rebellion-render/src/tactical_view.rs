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
use crate::tactical_backdrop::{wall_clock_seconds, BackdropOffsets};
use crate::tactical_assets::{
    original_hull_axes, OriginalTacticalCamera, TacticalAssetRenderer, TacticalDecalRenderObject,
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
const BATTLE_RESULTS_SCENE_X: f32 = 12.0;
const BATTLE_RESULTS_SCENE_Y: f32 = 13.0;
const BATTLE_RESULTS_RAIL_X: f32 = 412.0;
/// The battle windows' texts: pure red for the Alliance, pure green for the
/// Empire (A0 TAC-01-C010, TAC-07-C001, TAC-07-C002; `FUN_0044f860` passes
/// `0xff` or `0xff00`).
fn battle_window_text_color(player_alliance: bool) -> Color {
    if player_alliance {
        Color::from_rgba(255, 0, 0, 255)
    } else {
        Color::from_rgba(0, 255, 0, 255)
    }
}

/// The Battle Alert and Battle Summary windows' top left in the A0
/// captures: (62, 49) over the Alliance command center (TAC-01-C010,
/// TAC-07-C001: frame 10710 and rail 10820 at window x 412) and (125, 52)
/// over the Empire's (TAC-07-C002, the Umgul alert: frame 10711, rail
/// 10821 at (537, 52)). port: the original windows are draggable; these
/// are where they open.
const BATTLE_WINDOW_ALLIANCE: (f32, f32) = (62.0, 49.0);
const BATTLE_WINDOW_EMPIRE: (f32, f32) = (125.0, 52.0);

thread_local! {
    static BATTLE_WINDOW_ORIGIN: std::cell::Cell<(f32, f32)> =
        const { std::cell::Cell::new(BATTLE_WINDOW_ALLIANCE) };
}

/// Open the battle windows over the player's command center.
fn set_battle_window_origin(player_alliance: bool) {
    BATTLE_WINDOW_ORIGIN.with(|origin| {
        origin.set(if player_alliance {
            BATTLE_WINDOW_ALLIANCE
        } else {
            BATTLE_WINDOW_EMPIRE
        });
    });
}

fn battle_window_origin() -> (f32, f32) {
    BATTLE_WINDOW_ORIGIN.with(std::cell::Cell::get)
}

/// Selected-capital-ship display panel.
const TACTICAL_SELECTED_SHIP_PANEL: u32 = 1302;

/// Origin of the 149x236 right-hand panels (TACTICAL 1302-1310). The A0
/// captures of the original place every one of them at (482, 28); their
/// buttons, icons and meters sit one pixel right of and below the positions
/// the port first used (A0 TAC-03-C001, TAC-05-C012, TAC-06-C003).
const TACTICAL_PANEL_X: f32 = 482.0;
const TACTICAL_PANEL_Y: f32 = 28.0;

/// Meters of the selected capital ship in TACTICAL 1302. The left meter is
/// hull integrity and the right meter shield strength: in the A0 run of
/// TAC-03-C006/C005/C003 the right meter emptied first and the ship died
/// when the left one emptied. Both are 37x10; the remaining part is pure
/// blue and the lost part pure red for either faction (A0 TAC-03-C009).
const TACTICAL_SELECTED_HULL_METER: NativeRect = NativeRect::new(516.0, 109.0, 37.0, 10.0);
const TACTICAL_SELECTED_SHIELD_METER: NativeRect = NativeRect::new(585.0, 109.0, 37.0, 10.0);
const TACTICAL_METER_REMAINING: Color = Color::new(0.0, 0.0, 1.0, 1.0);
const TACTICAL_METER_LOST: Color = Color::new(1.0, 0.0, 0.0, 1.0);
/// A0 TAC-03-C001/C009: the portrait (TACTICAL 2007, 2026) matches every
/// opaque pixel at (508, 45).
const TACTICAL_SELECTED_SHIP_ART_X: f32 = 508.0;
const TACTICAL_SELECTED_SHIP_ART_Y: f32 = 45.0;
const TACTICAL_SUBSYSTEM_X: [f32; 5] = [492.0, 519.0, 546.0, 573.0, 600.0];
const TACTICAL_SUBSYSTEM_Y: f32 = 131.0;

/// The panel texts' Arial (the original's only face, `FUN_0060eed0`), drawn
/// with Liberation Sans, which shares its metrics. The fighter panel's
/// labels take table font 8 (12 px cell, `FUN_005e1fd0`) and the withdrawal
/// texts font 10 (14 px cell, `FUN_005f1050`). port: in the A0 captures their
/// capitals are 7 and 8 rows tall and their widths fit Liberation Sans at
/// 9.36 and 10.9 px, 0.78 of the cell (TAC-03-C016, TAC-02-C002,
/// TAC-06-C003), so the port draws them at those sizes.
const TACTICAL_FONT_8_EM: f32 = 9.36;
const TACTICAL_FONT_10_EM: f32 = 10.9;
/// Rows from a text's first capital row to its baseline (font 8 and 10).
const TACTICAL_FONT_8_CAPS: f32 = 7.0;
const TACTICAL_FONT_10_CAPS: f32 = 8.0;
/// Glyphs rasterize at this size and scale to the em, since macroquad
/// takes whole font sizes.
const TACTICAL_FONT_RASTER: f32 = 10.0;

thread_local! {
    static TACTICAL_ARIAL: std::cell::OnceCell<Option<Font>> =
        const { std::cell::OnceCell::new() };
}

thread_local! {
    static TACTICAL_ARIAL_BOLD: std::cell::OnceCell<Option<Font>> =
        const { std::cell::OnceCell::new() };
}

fn tactical_arial_bold() -> Option<Font> {
    TACTICAL_ARIAL_BOLD.with(|cell| {
        cell.get_or_init(|| load_ttf_font_from_bytes(crate::theme::ARIAL_BOLD).ok())
            .clone()
    })
}

fn tactical_arial() -> Option<Font> {
    TACTICAL_ARIAL.with(|cell| {
        cell.get_or_init(|| load_ttf_font_from_bytes(crate::theme::ARIAL_REGULAR).ok())
            .clone()
    })
}

/// Draw `text` with the panel font at `em`, its first capital row at
/// `cap_top` (native pixels).
fn draw_tactical_panel_text(
    canvas: TacticalCanvas,
    text: &str,
    x: f32,
    cap_top: f32,
    (em, caps): (f32, f32),
) {
    let font = tactical_arial();
    let (screen_x, screen_y) = canvas.point(x, cap_top + caps);
    draw_text_ex(
        text,
        screen_x,
        screen_y,
        TextParams {
            font: font.as_ref(),
            font_size: tactical_text_font_size(canvas),
            font_scale: em / TACTICAL_FONT_RASTER,
            font_scale_aspect: 1.0,
            color: WHITE,
            ..Default::default()
        },
    );
}

/// A task-force button's number: A0 TAC-02-C008 and TAC-03-C001 show
/// Task Force 1's "1" ten rows tall at rows 5..14, centred on the button,
/// black at rest and white while the button is down or its task force
/// selected (`FUN_0059be80` sets state 4's text colour to `0x2ffffff`).
/// port: the panel's Arial at 14 px gives the A0 glyph's height.
const TASK_FORCE_NUMBER: (f32, f32) = (14.0, 10.0);
const TASK_FORCE_NUMBER_CAP_TOP: f32 = 5.0;

fn task_force_number_color(active: bool) -> Color {
    if active {
        WHITE
    } else {
        BLACK
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Font sizes are small positive pixel counts."
)]
fn draw_task_force_number(canvas: TacticalCanvas, index: u8, active: bool) {
    let label = (index + 1).to_string();
    let (em, caps) = TASK_FORCE_NUMBER;
    let font = tactical_arial();
    // Rasterized at the em itself: a 1-pixel stem stays solid.
    let font_size = (em * canvas.scale).round().max(1.0) as u16;
    let font_scale = 1.0;
    let width = measure_text(&label, font.as_ref(), font_size, font_scale).width;
    let button = TASK_FORCE_BUTTONS[usize::from(index)];
    let (button_x, baseline) = canvas.point(button.x, TASK_FORCE_NUMBER_CAP_TOP + caps);
    draw_text_ex(
        &label,
        (button_x + (button.width * canvas.scale - width) * 0.5).round(),
        baseline.round(),
        TextParams {
            font: font.as_ref(),
            font_size,
            font_scale,
            font_scale_aspect: 1.0,
            color: task_force_number_color(active),
            ..Default::default()
        },
    );
}

const TACTICAL_FONT_8: (f32, f32) = (TACTICAL_FONT_8_EM, TACTICAL_FONT_8_CAPS);
const TACTICAL_FONT_10: (f32, f32) = (TACTICAL_FONT_10_EM, TACTICAL_FONT_10_CAPS);

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Font sizes are small positive pixel counts."
)]
fn tactical_text_font_size(canvas: TacticalCanvas) -> u16 {
    (TACTICAL_FONT_RASTER * canvas.scale).round().max(1.0) as u16
}

/// The battle view's status texts. port: their capitals are 12 rows tall
/// and "Battle Paused" spans 96 px in A0 TAC-01-C003; Liberation Sans at
/// 16.5 px, narrowed to 0.93, gives both (GDI's Arial runs narrower than
/// FreeType's at the same height, as the panel texts show).
const TACTICAL_STATUS_EM: f32 = 16.5;
const TACTICAL_STATUS_ASPECT: f32 = 0.93;
const TACTICAL_STATUS_CAP_TOP: f32 = 33.0;
const TACTICAL_STATUS_CAPS: f32 = 12.0;

fn tactical_pause_label_color(player_is_alliance: bool) -> Color {
    if player_is_alliance {
        Color::new(1.0, 0.0, 0.0, 1.0)
    } else {
        Color::new(0.0, 1.0, 0.0, 1.0)
    }
}

fn tactical_status_text_params(canvas: TacticalCanvas, color: Color) -> TextParams<'static> {
    TextParams {
        font: None,
        font_size: tactical_text_font_size(canvas),
        font_scale: TACTICAL_STATUS_EM / TACTICAL_FONT_RASTER,
        font_scale_aspect: TACTICAL_STATUS_ASPECT,
        color,
        ..Default::default()
    }
}

/// Native-pixel width of a status text.
fn tactical_status_text_width(canvas: TacticalCanvas, text: &str) -> f32 {
    let font = tactical_arial();
    let params = tactical_status_text_params(canvas, WHITE);
    measure_text(
        text,
        font.as_ref(),
        params.font_size,
        params.font_scale * params.font_scale_aspect,
    )
    .width
        / canvas.scale
}

fn draw_tactical_status_text(canvas: TacticalCanvas, text: &str, x: f32, color: Color) {
    let font = tactical_arial();
    let (screen_x, screen_y) = canvas.point(x, TACTICAL_STATUS_CAP_TOP + TACTICAL_STATUS_CAPS);
    draw_text_ex(
        text,
        screen_x,
        screen_y,
        TextParams {
            font: font.as_ref(),
            ..tactical_status_text_params(canvas, color)
        },
    );
}

/// TEXTTACT 56773-56780, the fighter panel's type labels in the order the
/// original tries the group's types `0x1d..0x24` (code at `0x5e1c80`;
/// `ghidra/notes/tactical-panel-text.md`). Index k pairs with portrait
/// 2030 + k, that is tactical ordinal 29 + k.
const TACTICAL_FIGHTER_TYPE_LABELS: [&str; 8] = [
    "X-wings:",
    "Y-wings:",
    "A-wings:",
    "B-wings:",
    "Fighters:",
    "Intrcpts:",
    "Bombers:",
    "Defenders:",
];

/// The fighter panel's count line for a group's squadrons: the first type
/// present, its squadron count, then the group's craft as
/// `trunc(hull / 8)` over `trunc(maximum hull / 8)`, each squadron at least
/// 1 (`FUN_005c6530`/`FUN_005ba330`, `FUN_005c6570`/`FUN_005ba380`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TacticalFighterCount {
    kind: usize,
    squadrons: usize,
    craft: u32,
    craft_maximum: u32,
}

impl TacticalFighterCount {
    fn line(self) -> String {
        format!(
            "{} {} {}/{}",
            TACTICAL_FIGHTER_TYPE_LABELS[self.kind], self.squadrons, self.craft, self.craft_maximum
        )
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Hull values are small nonnegative floats; the original truncates with __ftol."
)]
fn tactical_fighter_count<'a>(
    squadrons: impl Iterator<Item = &'a TacticalFighter> + Clone,
) -> Option<TacticalFighterCount> {
    let kind = |fighter: &TacticalFighter| {
        fighter
            .tactical_resource
            .and_then(|resource| resource.tactical_ordinal.checked_sub(29))
            .map(usize::from)
            .filter(|&k| k < TACTICAL_FIGHTER_TYPE_LABELS.len())
    };
    let first = squadrons.clone().filter_map(kind).min()?;
    let count = squadrons
        .clone()
        .filter(|fighter| kind(fighter) == Some(first))
        .count();
    let craft = |hull: f32| ((hull.max(0.0) / ORIGINAL_FIGHTER_HULL_PER_CRAFT) as u32).max(1);
    let current: u32 = squadrons
        .clone()
        .map(|fighter| craft(fighter.hull_current))
        .sum();
    let maximum: u32 = squadrons.map(|fighter| craft(fighter.hull_max)).sum();
    Some(TacticalFighterCount {
        kind: first,
        squadrons: count,
        craft: current,
        craft_maximum: maximum,
    })
}

/// The selected capital ship's task-force caption (A0 TAC-03-C016 reads
/// "Task Force # 1").
fn tactical_task_force_caption(group: u8) -> String {
    format!("Task Force # {}", u16::from(group) + 1)
}

/// Default ship icon size when no sprite is available.
const DEFAULT_SHIP_SIZE: f32 = 40.0;

/// Fighter squadron icon size.
const FIGHTER_SIZE: f32 = 16.0;

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

// ── The shared tooltip in the battle screens ────────────────────────────────
//
// The battle screens draw with macroquad, so they keep their own frame of
// the shared tip (`win_tooltip`, `FUN_006008e0`): its timers, placement and
// look are the galaxy view's; a control registers itself while the pointer
// is over it, and the frame draws the tip last.

#[derive(Default)]
struct TacticalTip {
    timer: crate::win_tooltip::TipTimer,
    hovered: Option<(u64, String)>,
    pointer: Option<(f32, f32)>,
}

thread_local! {
    static TACTICAL_TIP: std::cell::RefCell<TacticalTip> =
        std::cell::RefCell::new(TacticalTip::default());
}

/// Register the battle control `id` with its text this frame.
fn tactical_tool_text(id: u64, text: String) {
    TACTICAL_TIP.with(|tip| tip.borrow_mut().hovered = Some((id, text)));
}

/// Register the control at `rect` (native pixels, right and bottom edges
/// outside) while the pointer is over it and it is `enabled`, with string
/// `string` of `module`.
fn tactical_tool(
    canvas: TacticalCanvas,
    rect: NativeRect,
    id: u64,
    module: crate::win_tooltip::StringModule,
    string: u16,
    enabled: bool,
) {
    let (x, y) = mouse_position();
    let (x, y) = canvas.logical_pointer(x, y);
    if enabled && rect.contains(x, y) {
        if let Some(text) = crate::win_tooltip::module_string(module, string) {
            tactical_tool_text(id, tactical_tip_shown(&text));
        }
    }
}

/// The text a battle tip shows for a TEXTTACT string: the prefix-style
/// tooltip control drops '&' (`win_tooltip::shown_text`; A0
/// `a0-vm/2026-11-13-tactical-panel-tips/15`).
fn tactical_tip_shown(text: &str) -> String {
    crate::win_tooltip::shown_text(text)
}

/// Run the tip's timers on this frame's control and draw the tip over
/// everything.
fn paint_tactical_tip(canvas: TacticalCanvas) {
    let (x, y) = mouse_position();
    let pointer = canvas.logical_pointer(x, y);
    let pressed =
        is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right);
    let shown = TACTICAL_TIP.with(|tip| {
        let mut tip = tip.borrow_mut();
        #[cfg(feature = "interface-test-fixtures")]
        let was_shown = tip.timer.is_shown();
        #[cfg(not(feature = "interface-test-fixtures"))]
        let was_shown = true;
        let hovered = tip.hovered.take();
        let moved = tip.pointer.is_some_and(|last| last != pointer);
        tip.pointer = Some(pointer);
        tip.timer
            .step(
                get_time(),
                hovered
                    .as_ref()
                    .map(|(id, text)| (egui_macroquad::egui::Id::new(id), text.as_str())),
                pointer,
                moved,
                pressed,
            )
            .map(|(text, at)| (text.to_owned(), at, !was_shown))
    });
    let Some((text, at, newly_shown)) = shown else {
        return;
    };
    let font = tactical_arial();
    let font_size = tactical_text_font_size(canvas);
    let font_scale = crate::win_tooltip::FONT_EM / TACTICAL_FONT_RASTER;
    let width = measure_text(&text, font.as_ref(), font_size, font_scale).width / canvas.scale;
    let rect = crate::win_tooltip::tip_rect(at, width);
    if newly_shown {
        crate::win_tooltip::log_shown(&text, at, rect);
    }
    let color = |c: egui_macroquad::egui::Color32| Color::from_rgba(c.r(), c.g(), c.b(), 255);
    let (left, top) = canvas.point(rect.min.x, rect.min.y);
    let (w, h) = (rect.width() * canvas.scale, rect.height() * canvas.scale);
    let one = canvas.scale;
    draw_rectangle(left, top, w, h, color(crate::win_tooltip::BORDER));
    draw_rectangle(
        left + one,
        top + one,
        w - 2.0 * one,
        h - 2.0 * one,
        color(crate::win_tooltip::FILL),
    );
    // The ink starts 4 pixels in; the capitals start 5 pixels down and are
    // 9 rows tall, so the baseline is 14 pixels down (A0 tooltip/01.png).
    let (text_x, baseline) = canvas.point(rect.min.x + 4.0, rect.min.y + 14.0);
    draw_text_ex(
        &text,
        text_x,
        baseline,
        TextParams {
            font: font.as_ref(),
            font_size,
            font_scale,
            font_scale_aspect: 1.0,
            color: color(crate::win_tooltip::TEXT),
            ..Default::default()
        },
    );
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
    /// The last strategy-view frame, copied by the app at the end of each
    /// galaxy frame: the Battle Alert and the Battle Results windows sit on
    /// the live command center (A0 battle-clean/00, message-rail/
    /// battle-averam-result: its rail icons, droid and status bar), not on
    /// the bare shell bitmap.
    pub strategy_underlay: Option<Texture2D>,
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
    /// The selected-ship panel's order line last logged.
    logged_orders_line: Option<String>,
    /// Default and player-memorized camera states for resources 1040–1043.
    default_camera: Option<TacticalCameraBookmark>,
    memorized_camera: Option<TacticalCameraBookmark>,
    /// Keyboard-driven task-force or fighter-group chase target.
    camera_follow: Option<TacticalCameraFollow>,
    /// The navigation point a left click selected (state `0x66bb58`).
    selected_navigation_point: Option<TacticalWorldPosition>,
    /// Original Tactical Display faction-wireframe switches.
    pub highlight_alliance: bool,
    pub highlight_empire: bool,
    /// Active source tab in the post-battle Battle Results window.
    pub battle_result_tab: BattleResultTab,
    /// Active force family in the Alliance or Imperial result tab.
    pub battle_result_category: BattleResultCategory,
    /// The forces page's list's first visible pixel (its scroll bar's
    /// `+0xe0`, `FUN_0060a490`).
    battle_result_scroll: usize,
    /// A press held on the forces list's scroll bar.
    battle_result_scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// The original strategic Battle Alert owns entry until the player takes
    /// command, simulates, or attempts to retreat.
    battle_alert_open: bool,
    battle_alert_tab: BattleAlertTab,
    /// The first row the alert's list shows (the wheel scrolls it).
    battle_alert_scroll: usize,
    /// A press held on the alert list's scroll bar.
    battle_alert_scroll_hold: Option<crate::list_scroll_bar::Hold>,
    battle_alert_pressed: Option<BattleAlertControl>,
    /// The strategic world already consumed the auto-resolve result. Closing
    /// the result window must not apply the same losses a second time.
    strategic_results_applied: bool,
    /// The battle's report was already handed out (`take_summary_log`).
    summary_logged: bool,
    /// The summary was drawn over the tactical view (logged once).
    summary_over_tactical_logged: bool,
    /// Whether the authentic Battle Options panel owns the right-hand housing.
    battle_options_open: bool,
    /// Captured Battle Options control. Dispatch occurs only on a matching release.
    battle_options_pressed: Option<TacticalBattleOptionsControl>,
    /// Captured side-panel control, dispatched on a matching release: A0
    /// TAC-05-C013 holds Hammer down (1138) with the preview unchanged.
    command_control_pressed: Option<TacticalCommandControl>,
    /// The contents list's scroll, in pixels, and the ship it belongs to:
    /// another ship's list starts at its first unit.
    contents_scroll: (Option<usize>, f32),
    /// The native withdrawal confirmation replaces the right-hand options panel.
    withdraw_confirmation_open: bool,
    /// Captured confirmation control. Dispatch occurs only on a matching release.
    withdraw_confirmation_pressed: Option<TacticalWithdrawConfirmationControl>,
    /// The player handed command of their force to the tactical simulation.
    player_observing: bool,
    /// The Death Star backdrop resource last reported to the console.
    /// The Death Star decal's last logged resource and where it showed, in
    /// 16-pixel cells (`None` when off screen).
    logged_death_star_backdrop: Option<(u32, Option<(i32, i32)>)>,
    logged_death_star_beam: bool,
    /// Captured press on the source Death Star superlaser control.
    death_star_laser_pressed: bool,
    /// The next valid right-click assigns the superlaser target.
    death_star_targeting: bool,
    command_panel: TacticalCommandPanel,
    warmed_font_sizes: HashSet<u16>,
    render_original_participants: bool,
    render_original_starfield: bool,
    render_original_planet: bool,
    /// The second the battle scene was built, which seeds the backdrop
    /// decals' framing (`FUN_005d4d10`, `FUN_005f5750`). `None` keeps both
    /// frames at the middle of their ranges.
    backdrop_clock: Option<u32>,
    render_pyrotechnics: bool,
    /// The units' poses before the last step, drawn toward the current ones.
    previous_poses: Option<TacticalPoses>,
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
    /// Still fixtures only: hold the presentational effects while paused,
    /// which the original does not (see `advance_tactical_frame`).
    #[cfg(feature = "interface-test-fixtures")]
    freeze_paused_effects: bool,
}

/// The ships' positions and forwards and the fighter groups' positions at
/// one step.
#[derive(Debug, Clone, Default, PartialEq)]
struct TacticalPoses {
    /// The step these poses were taken before.
    tick: u32,
    ships: Vec<(TacticalWorldPosition, TacticalWorldVector)>,
    fighters: Vec<TacticalWorldPosition>,
}

impl TacticalPoses {
    /// These poses, when they were taken just before the session's latest
    /// step.
    fn before_latest_step(&self, session: &BattleSession) -> Option<&Self> {
        (self.tick + 1 == session.combat_tick).then_some(self)
    }

    fn of(session: &BattleSession) -> Self {
        Self {
            tick: session.combat_tick,
            ships: session
                .ships
                .iter()
                .map(|ship| (ship.source_position, ship.source_forward))
                .collect(),
            fighters: session
                .fighters
                .iter()
                .map(|fighter| fighter.source_position)
                .collect(),
        }
    }
}

/// port: the original moves its units every frame by the frame's
/// milliseconds (`0x5a9ec0`); the port steps them every 250 ms so that a
/// battle replays alike. Between steps a unit is drawn `fraction` of the
/// way from its last pose to its current one, the forward renormalized.
fn presented_pose(
    previous: Option<(TacticalWorldPosition, TacticalWorldVector)>,
    current: (TacticalWorldPosition, TacticalWorldVector),
    fraction: f32,
) -> (TacticalWorldPosition, TacticalWorldVector) {
    let Some((from, from_forward)) = previous else {
        return current;
    };
    let (to, to_forward) = current;
    let t = fraction.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * t;
    let position = TacticalWorldPosition {
        x: mix(from.x, to.x),
        y: mix(from.y, to.y),
        z: mix(from.z, to.z),
    };
    let forward = TacticalWorldVector {
        x: mix(from_forward.x, to_forward.x),
        y: mix(from_forward.y, to_forward.y),
        z: mix(from_forward.z, to_forward.z),
    }
    .normalized();
    let forward = if forward == TacticalWorldVector::ZERO {
        to_forward
    } else {
        forward
    };
    (position, forward)
}

impl Default for TacticalState {
    fn default() -> Self {
        Self {
            session: None,
            strategy_underlay: None,
            dragging_ship: None,
            drag_offset: (0.0, 0.0),
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
            navigation_sets_visible: [false; 4],
            logged_navigation_scene: None,
            logged_orders_line: None,
            default_camera: None,
            memorized_camera: None,
            camera_follow: None,
            selected_navigation_point: None,
            // A0 TAC-01-C003 (Alliance) and TAC-01-C002 (Empire): a battle
            // opens with both side highlights off.
            highlight_alliance: false,
            highlight_empire: false,
            battle_result_tab: BattleResultTab::Summary,
            battle_result_category: BattleResultCategory::CapitalShips,
            battle_result_scroll: 0,
            battle_result_scroll_hold: None,
            battle_alert_open: false,
            battle_alert_tab: BattleAlertTab::Summary,
            battle_alert_scroll: 0,
            battle_alert_scroll_hold: None,
            battle_alert_pressed: None,
            strategic_results_applied: false,
            summary_logged: false,
            summary_over_tactical_logged: false,
            battle_options_open: false,
            battle_options_pressed: None,
            command_control_pressed: None,
            contents_scroll: (None, 0.0),
            withdraw_confirmation_open: false,
            withdraw_confirmation_pressed: None,
            player_observing: false,
            logged_death_star_backdrop: None,
            logged_death_star_beam: false,
            death_star_laser_pressed: false,
            death_star_targeting: false,
            command_panel: TacticalCommandPanel::Display,
            warmed_font_sizes: HashSet::new(),
            render_original_participants: true,
            render_original_starfield: true,
            render_original_planet: true,
            backdrop_clock: None,
            render_pyrotechnics: true,
            previous_poses: None,
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
            #[cfg(feature = "interface-test-fixtures")]
            freeze_paused_effects: false,
        }
    }
}

impl TacticalState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seat each hull's units for its contents list (`FUN_0040b0e0` hands
    /// the battle a ship's characters, then its special forces
    /// (`FUN_00504040`), then its regiments (`FUN_00504c40`)), where the
    /// Fleet window seats them ([`crate::fleet_window::ship_cargo`]).
    pub fn board_contents(&mut self, world: &GameWorld, transport: &TroopTransportState) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut fleets = HashMap::new();
        for ship in &mut session.ships {
            let cargo = fleets
                .entry(ship.fleet)
                .or_insert_with(|| crate::fleet_window::ship_cargo(world, transport, ship.fleet));
            ship.contents = cargo
                .get(ship.fleet_ship_index)
                .map(|cargo| ship_contents_of(world, cargo))
                .unwrap_or_default();
        }
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
        // Every fleet of both sides at the system fights (FUN_0040c780).
        let session = BattleSession::new_for_fleets(
            world,
            system,
            &rebellion_core::abstract_battle::side_fleets(world, system, attacker),
            &rebellion_core::abstract_battle::side_fleets(world, system, defender),
            player_is_attacker,
            tick,
            rng_seed,
        )
        .session;
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
        self.logged_orders_line = None;
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
        self.highlight_alliance = false;
        self.highlight_empire = false;
        self.battle_result_tab = BattleResultTab::Summary;
        self.battle_result_category = BattleResultCategory::CapitalShips;
        self.battle_result_scroll = 0;
        self.battle_alert_open = true;
        self.battle_alert_tab = BattleAlertTab::Summary;
        self.battle_alert_scroll = 0;
        self.battle_alert_pressed = None;
        self.strategic_results_applied = false;
        self.summary_logged = false;
        self.summary_over_tactical_logged = false;
        self.battle_options_open = false;
        self.battle_options_pressed = None;
        self.command_control_pressed = None;
        self.contents_scroll = (None, 0.0);
        self.withdraw_confirmation_open = false;
        self.withdraw_confirmation_pressed = None;
        self.player_observing = false;
        self.death_star_laser_pressed = false;
        self.death_star_targeting = false;
        self.command_panel = TacticalCommandPanel::Display;
        self.render_original_participants = true;
        self.render_original_starfield = true;
        self.render_original_planet = true;
        self.backdrop_clock = Some(wall_clock_seconds());
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

    /// Hold the impact, projectile, field and superlaser effects while the
    /// battle is paused, so a paused fixture's frames stay still. The
    /// original keeps them running (`advance_tactical_frame`); this never
    /// enters a production build.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn freeze_paused_effects_for_fixture(&mut self) {
        self.freeze_paused_effects = true;
    }

    fn freezes_paused_effects(&self) -> bool {
        #[cfg(feature = "interface-test-fixtures")]
        {
            self.freeze_paused_effects
        }
        #[cfg(not(feature = "interface-test-fixtures"))]
        {
            false
        }
    }

    /// Pin the second that seeds the backdrop framing; `None` centres both
    /// frames.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn set_backdrop_clock_for_fixture(&mut self, clock: Option<u32>) {
        self.backdrop_clock = clock;
    }

    /// The backdrop frames' offsets for this battle.
    fn backdrop_offsets(&self, extent: f32) -> BackdropOffsets {
        self.backdrop_clock.map_or(BackdropOffsets::CENTRED, |clock| {
            BackdropOffsets::from_clock(clock, extent, self.render_original_planet)
        })
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
        let extent = session.source_layout.battle_extent;
        let mut task_force = 0_u8;
        for ship in &mut session.ships {
            if ship.alive && ship.is_attacker == session.player_is_attacker {
                // FUN_005d9640's first view shows the player's lane at the
                // corner of the view; spread the ships along their lane so
                // the browser can click each task force.
                let step = f32::from(task_force);
                ship.source_position.x =
                    -ship.source_position.z.signum() * (0.1 + 0.04 * step) * extent;
                ship.source_position.y = (0.3 - 0.02 * step) * extent;
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
                // Inside the first Imperial view of FUN_005d9640's camera.
                target.source_position.x = -20.0;
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
        // FUN_005d04e0's shot: a 1..100 draw at or under (hull / full hull
        // x torpedoes x 0.5333 + the rating) x 0.5.
        match outcome {
            TacticalTrenchRunOutcome::Success => {
                // Seed 15 yields a maneuver draw of 8 and a shot draw of 6;
                // six torpedoes at full hull and rating 9 give (3.2 + 9) x
                // 0.5, a threshold of 6.
                session.fighters[index].torpedo_strength =
                    session.fighters[index].torpedo_strength.max(6);
                session.trench_run_success_rating = 9.0;
                session.tactical_rng = OriginalTacticalRng::new(15);
            }
            TacticalTrenchRunOutcome::Failure => {
                // No torpedoes and rating 1: (0 + 1) x 0.5, a threshold of
                // 0, so every shot misses until the maneuver hits destroy
                // the group.
                session.fighters[index].torpedo_strength = 0;
                session.trench_run_success_rating = 1.0;
                session.tactical_rng = OriginalTacticalRng::new(1);
            }
        };
        if session.trench_run_remaining_ms.is_none() {
            begin_original_attack_order(session, &[], &[index], TacticalOrder::AttackDeathStar);
            // The fixture skips the approach to state 5 (FUN_005cfec0).
            session.commit_original_trench_run();
        }
        session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
    }

    /// A0 TAC-01-C013's end without its minutes of fire: the Alliance's
    /// hulls keep one hull point and no shield, so the first hit kills the
    /// last one and the battle ends by itself (`check_battle_end`).
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_last_alliance_ship_lost_fixture(&mut self) {
        // The journey pauses on the last ship's death: its fire runs on
        // under the pause, as in the original (A0 TAC-03-C004 run r6).
        self.freeze_paused_effects = false;
        let Some(session) = self.session.as_mut() else {
            return;
        };
        for ship in session.ships.iter_mut().filter(|ship| ship.identity.is_alliance) {
            ship.hull_current = 1;
            ship.shield = 0;
            ship.refresh_subsystem_condition();
        }
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
        self.battle_result_scroll = 0;
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
        // The fixture presents the executors' result without the three
        // turns the order waits for (event `0xa`), then the recovery
        // executor's turn (FUN_005cf980: state 1) and its event `0x2c`
        // (FUN_005b1e10: state 2) without the three after it.
        session.run_posted_order_executors();
        session.advance_recovery_executors();
        for queued in std::mem::take(&mut session.turn_events) {
            if matches!(queued.event, TacticalTurnEvent::FighterReserved { .. }) {
                session.run_turn_event(queued.event);
            } else {
                session.turn_events.push(queued);
            }
        }

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

            // Each ordered alone, in a force of its own (FUN_005aded0).
            session.ships[turning].task_force = TACTICAL_UNGROUPED_TASK_FORCE;
            session.ships[arriving].task_force = TACTICAL_UNGROUPED_TASK_FORCE;
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
            // 2.5 behind the carrier's underside (0x5b0b80), facing it, so
            // its one frame (0x5b8f70) brings it within the 2.0 gate.
            let below = session.ships[arriving]
                .source_collision_envelope
                .map_or(0.0, |envelope| envelope.vertical_radius);
            let returning = player_fighters[0];
            let docking = player_fighters[1];
            session.fighters[returning].fighter_group = 0;
            session.fighters[docking].fighter_group = 1;
            // The executors' install (FUN_005cf940), for each group, comes
            // before the states.
            session.begin_original_fighter_recovery(&[returning, docking]);
            session.fighters[returning].source_position = TacticalWorldPosition {
                x: carrier_position.x,
                y: carrier_position.y - below,
                z: carrier_position.z - 2.5 * forward_z,
            };
            session.fighters[returning].sublight_engine_power = 4.0;
            session.fighters[returning].order = TacticalOrder::Recover;
            session.fighters[returning].recovery_state = TacticalFighterRecoveryState::Returning;
            session.fighters[returning].recovery_target = Some(arriving);

            session.fighters[docking].source_position = carrier_position;
            session.fighters[docking].order = TacticalOrder::Recover;
            session.fighters[docking].recovery_state = TacticalFighterRecoveryState::Docking;
            session.fighters[docking].recovery_target = Some(arriving);
            if let Some(frame) = session.fighter_frames.get_mut(returning) {
                frame.forward = TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: forward_z,
                };
            }
            // The docking squadron's 0x1a runs now (FUN_005b8bd0: state 4),
            // as run_posted_order_executors does for the orders, so it is
            // taken in where it stands. Then the returning one flies, and the
            // executor's turn docks it within 2.0 (FUN_005cf980: state 3,
            // event 0x1a).
            session.run_turn_event(TacticalTurnEvent::FighterDocked {
                fighter: docking,
                carrier: arriving,
            });
            session.advance_recovering_fighters(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            session.advance_recovery_executors();

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
            // As above: the executors run without their three turns.
            session.run_posted_order_executors();

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

        // A ship whose jump has started drops out of the target lists
        // (`FUN_005cdf50` skips state 6).
        session.ships[first_enemy_ship].retreating = true;
        session.ships[first_enemy_ship].retreat_progress = ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        session.fighters[first_enemy_fighter].recovery_state =
            TacticalFighterRecoveryState::Recovered;
        refresh_original_attack_targets(session);
    }

    /// A0 TAC-03-C017: every hostile capital has jumped out, so the ship
    /// ordered to Attack Capital Ships holds no target and its line reads
    /// the order's word alone. The first player capital is selected.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_attack_target_lost_fixture(&mut self) {
        self.configure_attack_target_lifecycle_fixture();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        for ship in &mut session.ships {
            if ship.alive && ship.is_attacker != session.player_is_attacker {
                ship.retreating = true;
                ship.retreat_progress = ORIGINAL_TACTICAL_STEP_MILLISECONDS;
            }
        }
        refresh_original_attack_targets(session);
        let Some(first) = session
            .ships
            .iter()
            .position(|ship| ship.alive && ship.is_attacker == session.player_is_attacker)
        else {
            return;
        };
        for ship in &mut session.ships {
            ship.selected = false;
        }
        for fighter in &mut session.fighters {
            fighter.selected = false;
        }
        session.ships[first].selected = true;
        session.selected_ship = Some(first);
        session.selected_fighter_group = None;
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
                    effect.shown_frame = effect.frame;
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
            // A0 TAC-03-C004 shows frame 9 (3369; evidence
            // 2026-11-13-fireball). A machine that swaps every third render
            // call (FUN_005d41a0) shows it on the calls at 9, 10 and 11; the
            // call at 11 has taken the hull away.
            effect.frame = 12;
            effect.shown_frame = 9;
        }
        session.paused = true;
        self.command_panel = TacticalCommandPanel::Display;
    }

    /// Queue every recovered weapon-event variant so the muted browser journey
    /// can prove the TACTICAL.DLL weapon bank: 13000-13017 and 13030-13053,
    /// then the unit sounds 13054-13059, 13061, 13062 and 13064.
    #[cfg(feature = "interface-test-fixtures")]
    pub fn configure_tactical_audio_fixture(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.pending_audio_cues.clear();
        for event in [
            TacticalAudioEvent::LaserAtCapital,
            TacticalAudioEvent::Turbolaser,
            TacticalAudioEvent::IonAtCapital,
            TacticalAudioEvent::LaserAtAllianceFighter,
            TacticalAudioEvent::LaserAtImperialFighter,
            TacticalAudioEvent::IonAtFighter,
            TacticalAudioEvent::ShieldHit,
            TacticalAudioEvent::LaserShieldHit,
            TacticalAudioEvent::IonShieldHit,
            TacticalAudioEvent::HullHit,
            TacticalAudioEvent::LaserHullHit,
            TacticalAudioEvent::IonHullHit,
            TacticalAudioEvent::SuperlaserBeam,
            TacticalAudioEvent::ImperialFighterHum,
            TacticalAudioEvent::AllianceFighterHum,
            TacticalAudioEvent::LargeCapitalHum,
            TacticalAudioEvent::MediumCapitalHum,
            TacticalAudioEvent::SmallCapitalHum,
            TacticalAudioEvent::FighterLaunch,
            TacticalAudioEvent::FighterArrival,
            TacticalAudioEvent::CapitalArrival,
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
        // A point straight ahead: the force's anchor moves toward it and the
        // ship follows (0x005eb1a0, FUN_005b2f30). The force's other members
        // take forces of their own (FUN_005aded0), so the ship leads it.
        let group = session.ships[target].task_force;
        let side = session.ships[target].is_attacker;
        for (index, ship) in session.ships.iter_mut().enumerate() {
            if index != target && ship.is_attacker == side && ship.task_force == group {
                ship.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
            }
        }
        let ahead = session.ships[target].source_forward;
        let start = session.ships[target].source_position;
        session.ships[target].navigation_route.clear();
        session.ships[target].source_waypoint = Some(TacticalWorldPosition {
            x: start.x,
            y: start.y,
            z: start.z + ahead.z * 100.0,
        });
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

    /// The looping hum the application should hold now
    /// (`BattleSession::selection_hum`): none behind the Battle Alert or
    /// with no battle.
    pub fn selection_hum(&self) -> Option<rebellion_core::tactical::TacticalSelectionHum> {
        if self.battle_alert_open {
            return None;
        }
        self.session.as_ref().and_then(BattleSession::selection_hum)
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
        // The abstract battle ends a side only when it has no active unit
        // left (FUN_005ae9c0 words 1 and 3), so a loser's hull that is still
        // whole escaped (state 9, FUN_005af910). FUN_00516ef0 then sends the
        // loser's fleets off, and their departure records the system's
        // withdrawn side, +0x78 bits 2..3 (FUN_0050c6c0 -> FUN_0050a0b0),
        // which the report reads as +0x54 (FUN_0049b060).
        let loser_is_attacker = match winner {
            CombatWinner::Attacker => Some(false),
            CombatWinner::Defender => Some(true),
            CombatWinner::Draw => None,
        };
        if let Some(loser_is_attacker) = loser_is_attacker {
            for ship in session
                .ships
                .iter_mut()
                .filter(|ship| ship.is_attacker == loser_is_attacker && ship.alive)
            {
                ship.retreated = true;
            }
        }
        session.queue_battle_outcome_voice(winner);
        session.winner = Some(winner);
        session.phase = BattlePhase::Results;
        session.paused = true;
        session.selected_ship = None;
        session.selected_fighter_group = None;
        self.battle_result_tab = BattleResultTab::Summary;
        self.battle_result_category = BattleResultCategory::CapitalShips;
        self.battle_result_scroll = 0;
        self.strategic_results_applied = true;
        true
    }

    /// The Battle Summary's headline and body as the report builds them
    /// for the viewer (`FUN_0049b0f0`), once the battle has a result.
    #[must_use]
    pub fn battle_summary_lines(&self, world: &GameWorld) -> Option<Vec<String>> {
        self.session
            .as_ref()
            .filter(|session| session.phase == BattlePhase::Results)
            .map(|session| battle_result_summary_lines(session, world))
    }

    /// The report's lines once per battle, the first time it has its
    /// result (simulated, or played until a side has no unit left).
    pub fn take_summary_log(&mut self, world: &GameWorld) -> Option<Vec<String>> {
        if self.summary_logged {
            return None;
        }
        let lines = self.battle_summary_lines(world)?;
        self.summary_logged = true;
        Some(lines)
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

    /// Whether the battle has ended and its Battle Summary is open: from
    /// then on it is a window of the strategy screen (`FUN_0042a640`).
    #[must_use]
    pub fn battle_summary_open(&self) -> bool {
        !self.battle_alert_open
            && self
                .session
                .as_ref()
                .is_some_and(|session| session.phase == BattlePhase::Results)
    }

    /// The open Battle Summary window's rect on a screen of this size (the
    /// strategy screen's 640 x 480 fit), in screen pixels.
    #[must_use]
    pub fn battle_summary_screen_rect(&self, screen: (f32, f32)) -> Option<Rect> {
        let session = self.session.as_ref().filter(|_| self.battle_summary_open())?;
        let canvas = TacticalCanvas::new(screen.0, screen.1);
        let rect = battle_summary_window_rect(player_is_alliance(session));
        let (x, y) = canvas.point(rect.x, rect.y);
        Some(Rect::new(x, y, rect.w * canvas.scale, rect.h * canvas.scale))
    }

    /// Whether a screen point lies on the open Battle Summary window, on a
    /// screen of this size.
    #[must_use]
    pub fn battle_summary_contains_screen_point(
        &self,
        screen: (f32, f32),
        point: (f32, f32),
    ) -> bool {
        self.battle_summary_screen_rect(screen).is_some_and(|rect| {
            NativeRect::new(rect.x, rect.y, rect.w, rect.h).contains(point.0, point.1)
        })
    }

    /// Hold the pointer over the open Battle Summary from the strategy
    /// screen's egui windows and controls under it: the summary is drawn
    /// over them, after the egui pass. port: the summary stays on top of
    /// every strategy window (the windows' z-order across kinds is the
    /// port's own).
    pub fn battle_summary_input_hold(&self, ctx: &egui_macroquad::egui::Context) {
        use egui_macroquad::egui;
        let Some(rect) = self.battle_summary_screen_rect((screen_width(), screen_height())) else {
            return;
        };
        let screen =
            egui::Rect::from_min_size(egui::pos2(rect.x, rect.y), egui::vec2(rect.w, rect.h));
        egui::Area::new(egui::Id::new("battle_summary_input_hold"))
            .order(egui::Order::Tooltip)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.allocate_rect(screen, egui::Sense::click_and_drag());
            });
    }

    /// Whether the pre-battle source surface still owns tactical entry.
    #[must_use]
    pub const fn battle_alert_open(&self) -> bool {
        self.battle_alert_open
    }

    /// The original score the shown battle window plays: the Battle Alert's
    /// track by the viewer's side, or the Battle Summary's by the side and
    /// whether the viewer won (`crate::audio`). `None` during the tactical
    /// battle and outside one.
    #[must_use]
    pub fn screen_music(&self) -> Option<crate::audio::MusicTrack> {
        let session = self.session.as_ref()?;
        let viewer = player_is_alliance(session);
        if self.battle_alert_open {
            Some(crate::audio::battle_alert_music(viewer))
        } else if session.phase == BattlePhase::Results {
            Some(crate::audio::battle_summary_music(
                viewer,
                winner_is_alliance(session) == Some(viewer),
            ))
        } else {
            None
        }
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

/// Lower HUD controls, at the positions where A0 TAC-04-C005 matches each
/// TACTICAL.DLL bitmap pixel for pixel (transparent key colour left out).
const TACTICAL_HUD_CONTROLS: [TacticalHudControlSpec; 17] = [
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(0),
        rect: NativeRect::new(485.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[0],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(1),
        rect: NativeRect::new(524.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[1],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(2),
        rect: NativeRect::new(563.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[2],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::NavigationSet(3),
        rect: NativeRect::new(602.0, 272.0, 27.0, 27.0),
        hit_resource: resources::tactical::BTN_NAVIGATION_SET_NORMAL[3],
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::Pause,
        rect: NativeRect::new(561.0, 308.0, 28.0, 21.0),
        hit_resource: resources::tactical::BTN_PAUSE_RUNNING,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::ZoomIn,
        rect: NativeRect::new(491.0, 343.0, 24.0, 24.0),
        hit_resource: resources::tactical::BTN_CAMERA_ZOOM_IN_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::ZoomOut,
        rect: NativeRect::new(607.0, 343.0, 24.0, 24.0),
        hit_resource: resources::tactical::BTN_CAMERA_ZOOM_OUT_NORMAL,
    },
    // The target button is painted above the four overlapping D-pad arms and
    // therefore owns opaque pixels in their shared bounding region.
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraTarget,
        rect: NativeRect::new(549.0, 391.0, 23.0, 23.0),
        hit_resource: resources::tactical::BTN_CAMERA_TARGET_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraLeft,
        rect: NativeRect::new(497.0, 381.0, 42.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_LEFT_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraRight,
        rect: NativeRect::new(582.0, 381.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_RIGHT_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraUp,
        rect: NativeRect::new(539.0, 338.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_UP_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraDown,
        rect: NativeRect::new(540.0, 424.0, 43.0, 43.0),
        hit_resource: resources::tactical::BTN_CAMERA_DOWN_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraRecall,
        rect: NativeRect::new(486.0, 444.0, 43.0, 25.0),
        hit_resource: resources::tactical::BTN_CAMERA_RECALL_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::CameraMemorize,
        rect: NativeRect::new(594.0, 444.0, 43.0, 25.0),
        hit_resource: resources::tactical::BTN_CAMERA_MEMORIZE_NORMAL,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::HighlightAlliance,
        rect: NativeRect::new(515.0, 305.0, 30.0, 26.0),
        hit_resource: resources::tactical::DIM_ALLIANCE_SHIPS,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::HighlightEmpire,
        rect: NativeRect::new(483.0, 305.0, 30.0, 26.0),
        hit_resource: resources::tactical::DIM_EMPIRE_SHIPS,
    },
    TacticalHudControlSpec {
        control: TacticalHudControl::BattleOptions,
        rect: NativeRect::new(608.0, 309.0, 20.0, 20.0),
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
        rect: NativeRect::new(500.0, 47.0, 48.0, 34.0),
        normal_resource: resources::tactical::BTN_WITHDRAW_FROM_BATTLE_NORMAL,
        pressed_resource: resources::tactical::BTN_WITHDRAW_FROM_BATTLE_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::SimulateRemainder,
        rect: NativeRect::new(496.0, 121.0, 44.0, 25.0),
        normal_resource: resources::tactical::BTN_SIMULATE_REMAINDER_NORMAL,
        pressed_resource: resources::tactical::BTN_SIMULATE_REMAINDER_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::ObserveToggle,
        rect: NativeRect::new(569.0, 121.0, 45.0, 25.0),
        normal_resource: resources::tactical::BTN_OBSERVE_TOGGLE_NORMAL,
        pressed_resource: resources::tactical::BTN_OBSERVE_TOGGLE_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::GameOptions,
        rect: NativeRect::new(505.0, 212.0, 52.0, 33.0),
        normal_resource: resources::tactical::BTN_GAME_OPTIONS_NORMAL,
        pressed_resource: resources::tactical::BTN_GAME_OPTIONS_PRESSED,
    },
    TacticalBattleOptionsSpec {
        control: TacticalBattleOptionsControl::Close,
        rect: NativeRect::new(579.0, 212.0, 29.0, 33.0),
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
        rect: NativeRect::new(548.0, 230.0, 27.0, 25.0),
        normal_resource: resources::tactical::BTN_ASSIGN_CONFIRM_NORMAL,
        pressed_resource: resources::tactical::BTN_ASSIGN_CONFIRM_PRESSED,
    },
    TacticalWithdrawConfirmationSpec {
        control: TacticalWithdrawConfirmationControl::Cancel,
        rect: NativeRect::new(588.0, 230.0, 27.0, 25.0),
        normal_resource: resources::tactical::BTN_ASSIGN_CANCEL_NORMAL,
        pressed_resource: resources::tactical::BTN_ASSIGN_CANCEL_PRESSED,
    },
];

const DEATH_STAR_LASER_GAUGE_RECT: NativeRect = NativeRect::new(474.0, 0.0, 166.0, 25.0);
/// `FUN_005c4ed0` places the control at (0x89, 2) in the gauge window, 21x20;
/// A0 TAC-05-C017 matches all 381 opaque pixels of 1021 at (611, 2).
const DEATH_STAR_LASER_BUTTON_RECT: NativeRect = NativeRect::new(611.0, 2.0, 21.0, 20.0);
/// `FUN_005c4ed0` builds the meter at (0x22, 9), 0x5f by 6, with range 0..100.
const DEATH_STAR_LASER_FILL_RECT: NativeRect = NativeRect::new(508.0, 9.0, 95.0, 6.0);
/// The meter's colours, `FUN_0060e150` arguments `0x200ff00` (charge) and
/// `0x2ff0000` (rest) as COLORREFs: pure green on pure blue (A0 TAC-05-C020).
const DEATH_STAR_LASER_CHARGE_COLOR: Color = Color::new(0.0, 1.0, 0.0, 1.0);
const DEATH_STAR_LASER_EMPTY_COLOR: Color = Color::new(0.0, 0.0, 1.0, 1.0);

/// A0 TAC-02-C002: task-force slots every 29 px from x 60 (1007 matches at
/// 89, 118, ... 263), squadron buttons every 29 px from x 347.
const TASK_FORCE_BUTTONS: [NativeRect; 8] = [
    NativeRect::new(60.0, 2.0, 25.0, 17.0),
    NativeRect::new(89.0, 2.0, 25.0, 17.0),
    NativeRect::new(118.0, 2.0, 25.0, 17.0),
    NativeRect::new(147.0, 2.0, 25.0, 17.0),
    NativeRect::new(176.0, 2.0, 25.0, 17.0),
    NativeRect::new(205.0, 2.0, 25.0, 17.0),
    NativeRect::new(234.0, 2.0, 25.0, 17.0),
    NativeRect::new(263.0, 2.0, 25.0, 17.0),
];

/// `FUN_0059c030` makes the squadron buttons 25x15 (`0x19`, `0xf`), so the
/// bottom two rows of their 25x17 bitmaps never show: A0 TAC-02-C012 has the
/// backdrop 1000 there.
const FIGHTER_GROUP_BUTTONS: [NativeRect; 4] = [
    NativeRect::new(347.0, 2.0, 25.0, 15.0),
    NativeRect::new(376.0, 2.0, 25.0, 15.0),
    NativeRect::new(405.0, 2.0, 25.0, 15.0),
    NativeRect::new(434.0, 2.0, 25.0, 15.0),
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
            NativeRect::new(489.0, 38.0, 11.0, 53.0),
            art::BTN_PREVIOUS_CAPITAL_NORMAL,
        ),
        TacticalCommandControl::NextCapital => (
            NativeRect::new(614.0, 38.0, 11.0, 53.0),
            art::BTN_NEXT_CAPITAL_NORMAL,
        ),
        // A0 TAC-05-C012/C015: the left button (tooltip "Maneuvers
        // Tactics") opens the maneuvers panel, the right one "Missions".
        TacticalCommandControl::OpenManeuvers => (
            NativeRect::new(494.0, 238.0, 58.0, 22.0),
            art::BTN_MANEUVERS_TACTICS_NORMAL,
        ),
        TacticalCommandControl::OpenMissions => (
            NativeRect::new(562.0, 238.0, 58.0, 22.0),
            art::BTN_MISSIONS_NORMAL,
        ),
        TacticalCommandControl::LeftHook => (
            NativeRect::new(495.0, 187.0, 28.0, 28.0),
            art::BTN_LEFT_HOOK_NORMAL,
        ),
        TacticalCommandControl::RightHook => (
            NativeRect::new(525.0, 187.0, 28.0, 28.0),
            art::BTN_RIGHT_HOOK_NORMAL,
        ),
        TacticalCommandControl::Hammer => (
            NativeRect::new(495.0, 157.0, 28.0, 28.0),
            art::BTN_HAMMER_NORMAL,
        ),
        TacticalCommandControl::Anvil => (
            NativeRect::new(525.0, 157.0, 28.0, 28.0),
            art::BTN_ANVIL_NORMAL,
        ),
        TacticalCommandControl::HoldPosition => (
            NativeRect::new(557.0, 172.0, 28.0, 28.0),
            art::BTN_HOLD_POSITION_NORMAL,
        ),
        TacticalCommandControl::ToggleTactic => (
            NativeRect::new(593.0, 160.0, 24.0, 52.0),
            art::BTN_TACTIC_STAND_OFF,
        ),
        TacticalCommandControl::AttackCapitalShips => (
            NativeRect::new(500.0, 157.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_ATTACK_CAPITAL_SHIPS_ALLIANCE_NORMAL
            } else {
                art::BTN_ATTACK_CAPITAL_SHIPS_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::AttackFighters => (
            NativeRect::new(500.0, 187.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_ATTACK_FIGHTERS_ALLIANCE_NORMAL
            } else {
                art::BTN_ATTACK_FIGHTERS_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::Recover => (
            NativeRect::new(568.0, 157.0, 46.0, 26.0),
            if player_is_alliance {
                art::BTN_RECOVER_ALLIANCE_NORMAL
            } else {
                art::BTN_RECOVER_EMPIRE_NORMAL
            },
        ),
        TacticalCommandControl::AttackDeathStar => (
            NativeRect::new(568.0, 187.0, 46.0, 26.0),
            art::BTN_ATTACK_DEATH_STAR_NORMAL,
        ),
        TacticalCommandControl::Confirm => (
            NativeRect::new(548.0, 230.0, 27.0, 25.0),
            art::BTN_ASSIGN_CONFIRM_NORMAL,
        ),
        TacticalCommandControl::Cancel => (
            NativeRect::new(588.0, 230.0, 27.0, 25.0),
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
    gates: BattleOptionsGates,
    x: f32,
    y: f32,
) -> Option<TacticalBattleOptionsControl> {
    TACTICAL_BATTLE_OPTIONS_CONTROLS.iter().find_map(|spec| {
        if !gates.enabled(spec.control) {
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
    session.selected_fighter_group.is_some()
        && session
            .death_star
            .is_some_and(|death_star| death_star.is_attacker != session.player_is_attacker)
        && session.attack_death_star_open()
}

fn player_death_star(session: &BattleSession) -> Option<TacticalDeathStar> {
    session
        .death_star
        .filter(|death_star| death_star.is_attacker == session.player_is_attacker)
}

/// The charged part of the meter in pixels. port: whole pixels, rounded down.
fn death_star_laser_charge_width(charge: f32) -> f32 {
    (DEATH_STAR_LASER_FILL_RECT.width
        * (charge / ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE).clamp(0.0, 1.0))
    .floor()
}

/// The laser control's art. A0 TAC-05-C019 shows 1022 while the control is
/// armed; TAC-05-C018 shows 1023 as soon as the shot is committed, with the
/// meter empty, so a committed shot is not drawn as pressed.
fn death_star_laser_control_resource(ready: bool, armed: bool) -> u32 {
    use resources::tactical as art;
    if armed {
        art::DEATH_STAR_LASER_FIRED
    } else if ready {
        art::DEATH_STAR_LASER_READY
    } else {
        art::DEATH_STAR_LASER_LOADING
    }
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

/// `FUN_005a8a90` -> `FUN_005c7e80`: a unit of the side with a working
/// hyperdrive (`FUN_005b1b70`, engine power too) and no withdrawal begun.
fn tactical_withdraw_enabled(session: &BattleSession) -> bool {
    session.ships.iter().enumerate().any(|(index, ship)| {
        ship.alive
            && !ship.retreating
            && ship.is_attacker == session.player_is_attacker
            && rebellion_core::tactical::original_ship_hyperdrive_works(
                &session.ships,
                &session.field_effects,
                index,
            )
    })
}

/// Which Battle Options controls `FUN_005dab90` leaves enabled; it hides
/// the others, so the panel shows blank there. Withdraw needs a unit that
/// can jump (`FUN_00596a90`). Get Immediate Result and Observe / Take
/// Command go once either side has begun to withdraw (`FUN_00596ab0` ->
/// `FUN_005adde0`: a side commander's `+0x84` set, which `FUN_005ad530`
/// does): A0 TAC-06-C009 after a confirmed withdrawal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BattleOptionsGates {
    withdraw: bool,
    simulate: bool,
    observe: bool,
}

impl BattleOptionsGates {
    fn of(session: Option<&BattleSession>) -> Self {
        let withdrawing = session.is_some_and(|session| {
            session.withdrawal_ms.is_some() || session.ai_withdrawal_ms.is_some()
        });
        Self {
            withdraw: session.is_some_and(tactical_withdraw_enabled),
            simulate: !withdrawing,
            observe: !withdrawing,
        }
    }

    fn enabled(self, control: TacticalBattleOptionsControl) -> bool {
        match control {
            TacticalBattleOptionsControl::Withdraw => self.withdraw,
            TacticalBattleOptionsControl::SimulateRemainder => self.simulate,
            TacticalBattleOptionsControl::ObserveToggle => self.observe,
            TacticalBattleOptionsControl::GameOptions | TacticalBattleOptionsControl::Close => true,
        }
    }
}

/// The side highlight around a capital ship (FUN_0059f380): pure red for
/// the Alliance (A0 TAC-04-C006), pure green for the Empire (TAC-04-C007),
/// while that side's toggle is on.
fn tactical_highlight_color(is_alliance: bool, (alliance, empire): (bool, bool)) -> Option<Color> {
    match (is_alliance, alliance, empire) {
        (true, true, _) => Some(Color::new(1.0, 0.0, 0.0, 1.0)),
        (false, _, true) => Some(Color::new(0.0, 1.0, 0.0, 1.0)),
        _ => None,
    }
}

/// A0 VM 2026-10-27: an enemy's selection frame is red (255, 0, 0); ours
/// keeps the yellow one (`None`).
fn enemy_selection_frame_color(session: &BattleSession, ship: &TacticalShip) -> Option<Color> {
    (ship.is_attacker != session.player_is_attacker).then(|| Color::new(1.0, 0.0, 0.0, 1.0))
}

/// The enemy ship the player has selected, when the selection is one.
fn selected_enemy_ship(session: &BattleSession) -> Option<&TacticalShip> {
    session
        .selected_ship
        .and_then(|index| session.ships.get(index))
        .filter(|ship| ship.alive && ship.selected && ship.is_attacker != session.player_is_attacker)
}

#[derive(Debug, Clone, Copy)]
struct TacticalGroupHud<'a> {
    task_forces: [bool; 8],
    fighter_groups: [bool; 4],
    selected_task_force: Option<u8>,
    selected_fighter_group: Option<u8>,
    selected_ship_name: Option<&'a str>,
    /// The panel's ship is an enemy's (`0x66bba8`).
    selected_enemy: bool,
    /// The panel ship's own numbered group (`FUN_005b1480`), ours only.
    selected_ship_group: Option<u8>,
    selected_ship_hud: Option<u32>,
    selected_ship_hull: Option<(i32, i32)>,
    selected_ship_shield: Option<(i32, i32)>,
    selected_ship_subsystems: Option<TacticalSubsystemCondition>,
    selected_ship_contents: &'a [TacticalShipContent],
    selected_fighter_name: Option<&'a str>,
    selected_fighter_hud: Option<u32>,
    selected_fighter_count: Option<TacticalFighterCount>,
    selected_order: TacticalOrder,
    /// What the panel ship is doing, ours only (`FUN_005e4b40`).
    selected_ship_activity: Option<TacticalShipActivity<'a>>,
    /// The panel ship's index in the session.
    selected_ship_index: Option<usize>,
    /// What the panel's squadron is doing (`FUN_005e16a0`).
    selected_fighter_activity: Option<TacticalShipActivity<'a>>,
    selected_tactic: TacticalTactic,
}

/// The panel's order line beyond the order's word (`FUN_005e4b40` on the
/// unit's mover, `FUN_005a6460`): "Navigating" (TEXTTACT `0xdd8e`) while
/// its route (`+0x80`) holds a point, or "Disabled" (`0xdd91`) when its
/// speed (vt `+0x90`, `FUN_005b17f0`) is 0; else, holding a target
/// (`+0x98`), "Attacking" (`0xdd8d`) and the target's name after a space
/// (`FUN_005b1610`, `DAT_006a7f40`), except for an escort: the mover's
/// order code (`+0x68`, which the fighter panel `FUN_005e16a0` switches
/// on) 1 keeps "Escort" (`0xdde1`) before the escorted ship's name. A0
/// TAC-04-C002 reads "Navigating", A0 TAC-02/03 "Attacking Victory
/// Destroyer 1" and "Attacking Swift". The fighter panel (`FUN_005e16a0`)
/// writes the same line without the speed test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TacticalShipActivity<'a> {
    Navigating,
    Disabled,
    Attacking(&'a str),
    Escorting(&'a str),
}

impl<'a> TacticalShipActivity<'a> {
    fn line(self) -> String {
        match self {
            Self::Navigating => "Navigating".to_owned(),
            Self::Disabled => "Disabled".to_owned(),
            Self::Attacking(name) => format!("Attacking {name}"),
            Self::Escorting(name) => format!("Escort {name}"),
        }
    }

    fn of_fighter(session: &'a BattleSession, index: usize) -> Option<Self> {
        let fighter = session.fighters.get(index)?;
        if !fighter.navigation_route.is_empty() {
            return Some(Self::Navigating);
        }
        if fighter.order == TacticalOrder::Escort {
            return Some(Self::Escorting(
                &session.ships.get(fighter.escort_target?)?.name,
            ));
        }
        Some(Self::Attacking(attack_target_name(
            session,
            fighter.attack_target?,
        )?))
    }

    fn of(session: &'a BattleSession, index: usize) -> Option<Self> {
        let ship = session.ships.get(index)?;
        if !ship.navigation_route.is_empty() {
            let speed = original_tactical_mobility(&session.ships, &session.field_effects, index)
                .map_or(0.0, |mobility| mobility.effective_engine_power);
            return Some(if speed == 0.0 {
                Self::Disabled
            } else {
                Self::Navigating
            });
        }
        if ship.order == TacticalOrder::Escort {
            return Some(Self::Escorting(
                &session.ships.get(ship.escort_target?)?.name,
            ));
        }
        let target = ship
            .attack_target
            .or_else(|| ship.manual_targets.first().copied())?;
        Some(Self::Attacking(attack_target_name(session, target)?))
    }
}

fn attack_target_name(session: &BattleSession, target: TacticalAttackTarget) -> Option<&str> {
    match target {
        TacticalAttackTarget::CapitalShip(target) => Some(&session.ships.get(target)?.name),
        TacticalAttackTarget::FighterGroup(target) => Some(&session.fighters.get(target)?.name),
    }
}

impl<'a> TacticalGroupHud<'a> {
    /// The ship the selected-ship panel shows. The one-unit, several-unit
    /// and enemy states all show their unit (`FUN_005a1550`, `FUN_005a2280`,
    /// `FUN_005a1110`: `FUN_005c5f20`, `FUN_0051c9b0`) with no test of its
    /// group, so an ungrouped ship or a selection across forces shows too;
    /// only the "Task Force #" caption needs a group.
    fn panel_ship_name(&self) -> Option<&'a str> {
        self.selected_ship_name
    }

    /// The panel's caption (`FUN_005e4b40` `+0xac`): "Task Force #" and the
    /// unit's group (`FUN_005b1480`, TEXTTACT `0xdd9c`), or "Task Force
    /// Unassigned" (`0xdd9f`) for a unit in none, an enemy's included (A0
    /// VM 2026-10-27: the enemy "Haste" reads "Task Force Unassigned").
    fn panel_caption(&self) -> String {
        self.selected_ship_group.map_or_else(
            || "Task Force Unassigned".to_owned(),
            tactical_task_force_caption,
        )
    }

    /// The panel's order line: ours (`+0x168` the viewer's side) shows its
    /// order or what it is doing (`TacticalShipActivity`), an enemy's
    /// "Orders Unavailable" (`FUN_005e4b40`, TEXTTACT `0xddbf`).
    fn panel_orders_line(&self) -> String {
        if self.selected_enemy {
            return "Orders Unavailable".to_owned();
        }
        self.selected_ship_activity.map_or_else(
            || self.selected_order.label().to_owned(),
            TacticalShipActivity::line,
        )
    }

    /// The fighter panel's order line (`FUN_005e16a0`).
    fn fighter_orders_line(&self) -> String {
        self.selected_fighter_activity.map_or_else(
            || self.selected_order.label().to_owned(),
            TacticalShipActivity::line,
        )
    }

    fn from_session(session: &'a BattleSession) -> Self {
        let mut task_forces = [false; 8];
        let mut fighter_groups = [false; 4];
        for ship in &session.ships {
            if ship.alive && ship.is_attacker == session.player_is_attacker && ship.task_force < 8 {
                task_forces[usize::from(ship.task_force)] = true;
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
        // An enemy selection (`0x66bba8`) shows its ship like one of ours.
        // The panel's unit (`FUN_0051c9b0`): the clicked one when it is
        // still selected, else the first selected ship.
        let own_selected = |ship: &&TacticalShip| {
            ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
        };
        let selected_ship = session
            .selected_ship
            .and_then(|index| session.ships.get(index))
            .filter(own_selected)
            .or_else(|| session.ships.iter().find(own_selected))
            .or_else(|| selected_enemy_ship(session));
        let selected_fighter = session.selected_fighter_group.and_then(|group| {
            session.fighters.iter().find(|fighter| {
                fighter.alive
                    && fighter.selected
                    && fighter.is_attacker == session.player_is_attacker
                    && fighter.fighter_group == group
            })
        });
        let selected_fighter_count = session.selected_fighter_group.and_then(|group| {
            tactical_fighter_count(session.fighters.iter().filter(|fighter| {
                fighter.alive
                    && fighter.is_attacker == session.player_is_attacker
                    && fighter.fighter_group == group
            }))
        });
        Self {
            task_forces,
            fighter_groups,
            selected_task_force,
            selected_fighter_group: session.selected_fighter_group,
            selected_fighter_count,
            selected_ship_name: selected_ship.map(|ship| ship.name.as_str()),
            selected_enemy: selected_ship
                .is_some_and(|ship| ship.is_attacker != session.player_is_attacker),
            selected_ship_group: selected_ship
                .filter(|ship| ship.is_attacker == session.player_is_attacker)
                .map(|ship| ship.task_force)
                .filter(|&group| group != TACTICAL_UNGROUPED_TASK_FORCE),
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
            selected_ship_index: selected_ship.and_then(|ship| {
                session
                    .ships
                    .iter()
                    .position(|each| std::ptr::eq(each, ship))
            }),
            selected_ship_activity: selected_ship
                .filter(|ship| ship.is_attacker == session.player_is_attacker)
                .and_then(|ship| {
                    session
                        .ships
                        .iter()
                        .position(|each| std::ptr::eq(each, ship))
                })
                .and_then(|index| TacticalShipActivity::of(session, index)),
            selected_fighter_activity: selected_fighter
                .and_then(|fighter| {
                    session
                        .fighters
                        .iter()
                        .position(|each| std::ptr::eq(each, fighter))
                })
                .and_then(|index| TacticalShipActivity::of_fighter(session, index)),
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

/// `FUN_00597610`'s contents icons, as (key, TACTICAL bitmap): the key is
/// a unit's TEXTSTRA name id less `0x2000` (`FUN_005975d0`), which is its
/// STRATEGY mini's id less `0x4000` (the Alliance Army Regiment, name
/// `0x2441`, mini 17473, takes 2301: A0 TAC-03-C012).
const TACTICAL_CONTENT_ICONS: [(u16, u32); 79] = [
    (0x840, 2234),
    (0x841, 2241),
    (0x842, 2230),
    (0x843, 2255),
    (0x850, 2229),
    (0x880, 2214),
    (0x881, 2260),
    (0xa40, 2201),
    (0xa41, 2204),
    (0xa42, 2205),
    (0xa43, 2206),
    (0xa44, 2210),
    (0xa45, 2231),
    (0xa46, 2250),
    (0xa47, 2202),
    (0xa48, 2212),
    (0xa49, 2217),
    (0xa4a, 2249),
    (0xa4b, 2209),
    (0xa4c, 2220),
    (0xa4d, 2225),
    (0xa4e, 2235),
    (0xa4f, 2237),
    (0xa50, 2245),
    (0xa51, 2254),
    (0xa52, 2256),
    (0xa53, 2263),
    (0xa54, 2264),
    (0xa55, 2223),
    (0xa56, 2203),
    (0xa57, 2248),
    (0xa58, 2243),
    (0xa59, 2232),
    (0xa80, 2222),
    (0xa81, 2244),
    (0xa82, 2247),
    (0xa83, 2261),
    (0xa84, 2266),
    (0xa85, 2207),
    (0xa86, 2211),
    (0xa87, 2215),
    (0xa88, 2216),
    (0xa89, 2219),
    (0xa8a, 2218),
    (0xa8b, 2226),
    (0xa8c, 2236),
    (0xa8d, 2239),
    (0xa8e, 2242),
    (0xa8f, 2246),
    (0xa90, 2253),
    (0xa91, 2258),
    (0xa92, 2265),
    (0xa93, 2208),
    (0xa94, 2257),
    (0xa95, 2228),
    (0xa96, 2251),
    (0xa97, 2240),
    (0xa98, 2221),
    (0xa99, 2233),
    (0xa9a, 2227),
    (0xa9b, 2262),
    (0x540, 2308),
    (0x541, 2309),
    (0x542, 2318),
    (0x543, 2302),
    (0x580, 2313),
    (0x581, 2305),
    (0x582, 2303),
    (0x583, 2312),
    (0x441, 2301),
    (0x440, 2310),
    (0x442, 2314),
    (0x443, 2316),
    (0x444, 2311),
    (0x480, 2317),
    (0x481, 2307),
    (0x482, 2306),
    (0x483, 2315),
    (0x484, 2304),
];

/// The contents list's icon for a unit with STRATEGY mini `mini`.
#[must_use]
pub fn tactical_content_icon(mini: u32) -> Option<u32> {
    let key = u16::try_from(mini.checked_sub(0x4000)?).ok()?;
    TACTICAL_CONTENT_ICONS
        .iter()
        .find(|(each, _)| *each == key)
        .map(|&(_, icon)| icon)
}

/// The selected ship's contents list (`FUN_005e4a30`): 124 by 45 at panel
/// (13, 126), 32-pixel cells (A0 VM a0-vm/coruscant-676/05: icons from
/// (495, 154) every 32 pixels, the fourth clipped at x 619).
const CONTENTS_LIST: NativeRect = NativeRect::new(495.0, 154.0, 124.0, 45.0);
const CONTENTS_CELL: f32 = 32.0;
/// Its scroll bar's row (`FUN_0060a560` with TACTICAL 1350's set): the
/// arrows 1351 / 1352 at x 495 and 1353 / 1354 at x 599, 20 wide, the
/// thumb 1355, 1356 and 1357 on the track between (A0 VM coruscant-676/05).
const CONTENTS_BAR_Y: f32 = 189.0;
const CONTENTS_BAR: NativeRect = NativeRect::new(495.0, 187.0, 124.0, 12.0);
const CONTENTS_LEFT_ARROW: NativeRect = NativeRect::new(495.0, 189.0, 20.0, 8.0);
const CONTENTS_RIGHT_ARROW: NativeRect = NativeRect::new(599.0, 189.0, 20.0, 8.0);
const CONTENTS_TRACK: (f32, f32) = (515.0, 84.0);

/// The list's content width and how far it scrolls.
fn contents_extent(count: usize) -> (f32, f32) {
    let content = count as f32 * CONTENTS_CELL;
    (content, (content - CONTENTS_LIST.width).max(0.0))
}

/// The thumb's left edge and width: the track times the shown share of the
/// list (A0 VM coruscant-676/05: four icons, 128 wide, give 81 of 84).
fn contents_thumb(count: usize, scroll: f32) -> Option<(f32, f32)> {
    let (content, range) = contents_extent(count);
    if range <= 0.0 {
        return None;
    }
    let (track_x, track) = CONTENTS_TRACK;
    let width = (track * CONTENTS_LIST.width / content).floor();
    Some((track_x + ((track - width) * scroll / range).round(), width))
}

/// One ship's contents list, in `FUN_0040b0e0`'s order, each unit with
/// its STRATEGY mini.
fn ship_contents_of(
    world: &GameWorld,
    cargo: &crate::fleet_window::ShipCargo,
) -> Vec<TacticalShipContent> {
    let characters = cargo.characters.iter().filter_map(|&key| {
        let character = world
            .characters
            .get(key)
            .filter(|character| !character.is_killed)?;
        Some(TacticalShipContent {
            label: character.name.clone(),
            resource_id: tactical_character_content_resource(character.dat_id, character.is_major)?,
        })
    });
    let special_forces = cargo.special_forces.iter().filter_map(|&key| {
        let (resource_id, label) =
            crate::system_window::special_force_mini(world.special_forces.get(key)?.class_dat_id)?;
        Some(TacticalShipContent {
            label: label.to_owned(),
            resource_id,
        })
    });
    let regiments = cargo.regiments.iter().filter_map(|&key| {
        let (resource_id, label) =
            crate::system_window::troop_mini(world.troops.get(key)?.class_dat_id)?;
        Some(TacticalShipContent {
            label: label.to_owned(),
            resource_id,
        })
    });
    characters.chain(special_forces).chain(regiments).collect()
}

/// The scroll of `ship`'s list: its stored one, or 0 for another ship's.
fn contents_scroll_of((ship_of, scroll): (Option<usize>, f32), ship: Option<usize>) -> f32 {
    if ship.is_some() && ship_of == ship {
        scroll
    } else {
        0.0
    }
}

/// The list one arrow click away: a cell (`0x20`, the list's step).
fn scrolled_contents(count: usize, scroll: f32, right: bool) -> f32 {
    let (_, range) = contents_extent(count);
    let step = if right { CONTENTS_CELL } else { -CONTENTS_CELL };
    (scroll + step).clamp(0.0, range)
}

/// A contents icon as the 8-bit screen shows it: the list paints it with
/// GDI, which matches each colour to the battle's palette (A0 VM
/// coruscant-676/05 shows the Stormtroopers' entry 199, (238, 142, 142) in
/// the bitmap, as the palettes' (194, 158, 149)); the HUD's buttons keep
/// their indices instead. Without the palette, the bitmap's own colours.
fn draw_contents_icon(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    icon: u32,
    (x, y): (f32, f32),
    palette: Option<&(u32, Vec<[u8; 3]>)>,
) {
    let Some((key, colours)) = palette else {
        draw_tactical_bitmap(cache, icon, canvas, x, y);
        return;
    };
    let Some(texture) =
        cache.get_macroquad_in_screen_palette(DllSource::Tactical, icon, *key, colours)
    else {
        return;
    };
    let (screen_x, screen_y) = canvas.point(x, y);
    draw_texture_ex(
        texture,
        screen_x,
        screen_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(
                texture.width() * canvas.scale,
                texture.height() * canvas.scale,
            )),
            ..Default::default()
        },
    );
}

/// The selected ship's contents list (`FUN_005e4a30`): each unit's icon
/// in its cell, clipped to the list, and the scroll bar once they
/// overflow it.
fn draw_tactical_contents(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    contents: &[TacticalShipContent],
    scroll: f32,
    palette: Option<&(u32, Vec<[u8; 3]>)>,
) {
    let list = CONTENTS_LIST;
    let (left, top) = canvas.point(list.x, list.y);
    set_tactical_aperture_clip(Some(NativeRect::new(
        left,
        top,
        list.width * canvas.scale,
        CONTENTS_CELL * canvas.scale,
    )));
    for (index, content) in contents.iter().enumerate() {
        if let Some(icon) = tactical_content_icon(content.resource_id) {
            draw_contents_icon(
                cache,
                canvas,
                icon,
                (list.x + index as f32 * CONTENTS_CELL - scroll, list.y),
                palette,
            );
        }
    }
    set_tactical_aperture_clip(None);
    let Some((thumb_x, thumb_width)) = contents_thumb(contents.len(), scroll) else {
        return;
    };
    // The bar's window is black under its arrows and thumb (A0 VM
    // coruscant-676/05: rows 187..198 from x 495 to 618, and the track
    // the thumb leaves).
    let (bar_x, bar_y) = canvas.point(CONTENTS_BAR.x, CONTENTS_BAR.y);
    draw_rectangle(
        bar_x,
        bar_y,
        CONTENTS_BAR.width * canvas.scale,
        CONTENTS_BAR.height * canvas.scale,
        BLACK,
    );
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    let held = |arrow: NativeRect| is_mouse_button_down(MouseButton::Left) && arrow.contains(x, y);
    for (arrow, normal) in [(CONTENTS_LEFT_ARROW, 1351), (CONTENTS_RIGHT_ARROW, 1353)] {
        let id = if held(arrow) { normal + 1 } else { normal };
        draw_tactical_bitmap(cache, id, canvas, arrow.x, arrow.y);
    }
    // The thumb: its left cap, the middle tiled from 4 pixels in, and the
    // right cap on its last 5 (A0 VM coruscant-676/05: 1355 at 515, 1356
    // from 519, 1357 at 591 for an 81-wide thumb).
    let right_cap = thumb_x + thumb_width - 5.0;
    let (clip_x, clip_y) = canvas.point(thumb_x + 4.0, CONTENTS_BAR_Y);
    set_tactical_aperture_clip(Some(NativeRect::new(
        clip_x,
        clip_y,
        (right_cap - thumb_x - 4.0).max(0.0) * canvas.scale,
        8.0 * canvas.scale,
    )));
    let mut tile = thumb_x + 4.0;
    while tile < right_cap {
        draw_tactical_bitmap(cache, 1356, canvas, tile, CONTENTS_BAR_Y);
        tile += 10.0;
    }
    set_tactical_aperture_clip(None);
    draw_tactical_bitmap(cache, 1355, canvas, thumb_x, CONTENTS_BAR_Y);
    draw_tactical_bitmap(cache, 1357, canvas, right_cap, CONTENTS_BAR_Y);
}

/// Right-hand panel while the display (not an order panel) is up: the
/// fighter panel, the selected-ship panel, or with nothing selected the
/// player's emblem (A0 TAC-04-C005, TAC-01-C002).
fn tactical_display_panel_resource(
    fighter_selected: bool,
    ship_selected: bool,
    player_is_alliance: bool,
) -> u32 {
    use resources::tactical as art;
    if fighter_selected {
        1307
    } else if ship_selected {
        TACTICAL_SELECTED_SHIP_PANEL
    } else if player_is_alliance {
        art::ALLIANCE_EMBLEM_PANEL
    } else {
        art::EMPIRE_EMBLEM_PANEL
    }
}

/// Width of a meter's blue remaining part; the rest of the meter is red.
/// port: rounding to the nearest pixel is not read from the original.
fn tactical_meter_remaining_width(width: f32, fraction: f32) -> f32 {
    (width * fraction.clamp(0.0, 1.0)).round()
}

fn draw_tactical_selected_meter(canvas: TacticalCanvas, rect: NativeRect, fraction: f32) {
    let (x, y) = canvas.point(rect.x, rect.y);
    let width = rect.width * canvas.scale;
    let height = rect.height * canvas.scale;
    let remaining = tactical_meter_remaining_width(width, fraction);
    draw_rectangle(x, y, remaining, height, TACTICAL_METER_REMAINING);
    draw_rectangle(
        x + remaining,
        y,
        width - remaining,
        height,
        TACTICAL_METER_LOST,
    );
}

fn selected_player_task_force(session: &BattleSession) -> Option<u8> {
    if session.selected_fighter_group.is_some() {
        return None;
    }
    if session.task_force_selection.is_some() {
        return session.task_force_selection;
    }
    let mut selected = session.ships.iter().filter(|ship| {
        ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker
    });
    let group = selected.next()?.task_force;
    (group != TACTICAL_UNGROUPED_TASK_FORCE && selected.all(|ship| ship.task_force == group))
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
/// Space (`FUN_0059b730`, key `0x20`, the selection state's slot `+0x1c`):
/// from the selection's point, its one ship's position (`0x005a21e0`) or
/// its members' centroid (`0x005a3250` for several ships, `0x005a4180` for
/// a task force), the nearest enemy capital ship in the fight, else the
/// nearest enemy squadron (`FUN_005aa970`), becomes the target as by a
/// right click without Ctrl (slot `+0xc` with 0). An enemy selection or
/// none does nothing (`0x006158b0`). A fighter group searches from its
/// squadrons' centroid (`0x005a50d0`).
fn attack_nearest_enemy(session: &mut BattleSession) -> Option<TacticalFocusAssignment> {
    let group = session.selected_fighter_group;
    let player_is_attacker = session.player_is_attacker;
    let positions = match group {
        // `0x005a50d0`: a fighter group's members.
        Some(group) => session
            .fighters
            .iter()
            .filter(|fighter| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.is_attacker == player_is_attacker
                    && fighter.fighter_group == group
            })
            .map(|fighter| fighter.source_position)
            .collect::<Vec<_>>(),
        None => session
            .ships
            .iter()
            .filter(|ship| ship.alive && ship.selected && ship.is_attacker == player_is_attacker)
            .map(|ship| ship.source_position)
            .collect(),
    };
    let mut sum = TacticalWorldPosition { x: 0.0, y: 0.0, z: 0.0 };
    let mut count = 0.0_f32;
    for position in positions {
        sum.x += position.x;
        sum.y += position.y;
        sum.z += position.z;
        count += 1.0;
    }
    if count <= 0.0 {
        return None;
    }
    // The sum scaled by 1 / count (`DAT_0066bb54`).
    let scale = 1.0 / count;
    let point = TacticalWorldPosition {
        x: scale * sum.x,
        y: scale * sum.y,
        z: scale * sum.z,
    };
    let enemy = !session.player_is_attacker;
    let target = match session
        .nearest_unit_of_side(point, enemy, false)
        .or_else(|| session.nearest_unit_of_side(point, enemy, true))?
    {
        TacticalUnitRef::Ship(index) => TacticalAttackTarget::CapitalShip(index),
        TacticalUnitRef::Fighter(index) => TacticalAttackTarget::FighterGroup(index),
    };
    if group.is_some() {
        let members = assign_selected_fighter_group_target(session, target, false);
        return (members > 0).then(|| TacticalFocusAssignment::Assigned {
            object_ids: Vec::new(),
            maximum_targets: 1,
        });
    }
    Some(assign_selected_focus_target(session, target, false))
}

fn assign_selected_capital_focus_target(
    session: &mut BattleSession,
    target_idx: usize,
    append: bool,
) -> TacticalFocusAssignment {
    assign_selected_focus_target(session, TacticalAttackTarget::CapitalShip(target_idx), append)
}

/// The lowest-indexed enemy squadron still flying among the projected hits.
fn enemy_squadron_hit(
    session: &BattleSession,
    hits: impl Iterator<Item = (usize, bool)>,
) -> Option<usize> {
    hits.filter_map(|(index, hit)| {
        let fighter = session.fighters.get(index)?;
        (hit && fighter.alive
            && fighter.squad_count > 0
            && fighter.is_attacker != session.player_is_attacker)
            .then_some(index)
    })
    .min()
}

/// The fighter group state's slot `+0xc` (`0x005a4bd0`) for an enemy unit:
/// without Ctrl, the group's attack voice (`0x44`/`0xbe` plus its slot),
/// order 0 (slot `+100`) and the unit as its target (`FUN_005a8fc0`); with
/// Ctrl, a target it already holds (`FUN_005ca310`) is dropped
/// (`FUN_005a8e80`) and another one is added. port: a squadron holds one
/// target, so a Ctrl click on a new one replaces it. The voice needs the
/// force's route (`+0x80`, `FUN_005cb920`) empty, its slot `+0xc` being
/// the list's first point (`0x5f6590` -> `FUN_005f5060`); a Ctrl click on
/// a new target also needs its targets (`+0x98`, `FUN_005cb950`) empty
/// (slot `+0x10`, `0x5cbca0`). Returns the members it changed.
fn assign_selected_fighter_group_target(
    session: &mut BattleSession,
    target: TacticalAttackTarget,
    append: bool,
) -> usize {
    let Some(group) = session.selected_fighter_group else {
        return 0;
    };
    let player_is_attacker = session.player_is_attacker;
    let members = (0..session.fighters.len())
        .filter(|&index| {
            let fighter = &session.fighters[index];
            fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker == player_is_attacker
                && fighter.fighter_group == group
        })
        .collect::<Vec<_>>();
    if members.is_empty() {
        return 0;
    }
    let held = members
        .iter()
        .any(|&index| session.fighters[index].attack_target == Some(target));
    let route_empty = session.fighters[members[0]].navigation_route.is_empty();
    let targets_empty = members
        .iter()
        .all(|&index| session.fighters[index].attack_target.is_none());
    let voice = route_empty && (!append || (!held && targets_empty));
    for &index in &members {
        let fighter = &mut session.fighters[index];
        if append && held {
            fighter.attack_target = None;
        } else {
            fighter.order = TacticalOrder::None;
            fighter.escort_target = None;
            fighter.attack_target = Some(target);
        }
    }
    if voice {
        let faction = session.player_voice_faction();
        session
            .pending_voice_cues
            .push(TacticalVoiceCue::fighter_attack(faction, group));
    }
    members.len()
}

/// A capital selection's route voice (`0x005a1f30`, `0x005a3f00`): a task
/// force's line (`0x22`/`0x9c` plus its slot, `FUN_005a0300`), else the
/// generic one (`0x21`/`0x9b`). Without Ctrl it always plays; with Ctrl
/// only for a point not yet on the route, and only while the route
/// (`+0x80`, slot `+0xc` its first point, `FUN_005f5060`) is empty.
/// Several selected ships first form a task force
/// ([`route_selected_ships_as_a_force`]).
fn queue_capital_route_voice(
    session: &mut BattleSession,
    point: TacticalWorldPosition,
    control: bool,
) {
    let player_is_attacker = session.player_is_attacker;
    let selected = session
        .ships
        .iter()
        .filter(|ship| ship.selected && ship.alive && ship.is_attacker == player_is_attacker)
        .collect::<Vec<_>>();
    let Some(first) = selected.first() else {
        return;
    };
    if control && (first.navigation_route.contains(&point) || !first.navigation_route.is_empty())
    {
        return;
    }
    let group = session
        .task_force_selection
        .unwrap_or(TACTICAL_UNGROUPED_TASK_FORCE);
    let faction = session.player_voice_faction();
    session
        .pending_voice_cues
        .push(TacticalVoiceCue::capital_maneuver(faction, group));
}

/// The fighter group state's slot `+0x10` (`0x005a4e50`) for a navigation
/// point. Without Ctrl, the group's line (`0x2a`/`0xa4` plus its slot),
/// order 0 and the point as its whole route (`FUN_005a8f70`). With Ctrl, a
/// point already on the route (`FUN_005ca330`) is dropped (`FUN_005a8ed0`)
/// and another one added (`FUN_005a8f20`), with the line only while the
/// route (`+0x80`, slot `+0xc`) is empty. Returns the members it changed.
fn assign_selected_fighter_group_route(
    session: &mut BattleSession,
    point: TacticalWorldPosition,
    append: bool,
) -> usize {
    let Some(group) = session.selected_fighter_group else {
        return 0;
    };
    let player_is_attacker = session.player_is_attacker;
    let members = (0..session.fighters.len())
        .filter(|&index| {
            let fighter = &session.fighters[index];
            fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker == player_is_attacker
                && fighter.fighter_group == group
        })
        .collect::<Vec<_>>();
    let Some(&first) = members.first() else {
        return 0;
    };
    let held = append && session.fighters[first].navigation_route.contains(&point);
    let voice = !append || session.fighters[first].navigation_route.is_empty();
    for &index in &members {
        let fighter = &mut session.fighters[index];
        if held {
            fighter.navigation_route.retain(|each| *each != point);
        } else if append {
            fighter.navigation_route.push(point);
        } else {
            fighter.order = TacticalOrder::None;
            fighter.escort_target = None;
            fighter.navigation_route.clear();
            fighter.navigation_route.push(point);
        }
    }
    if voice && !held {
        let faction = session.player_voice_faction();
        session
            .pending_voice_cues
            .push(TacticalVoiceCue::fighter_maneuver(faction, group));
    }
    members.len()
}

/// A Ctrl click on a capital ship, left or right (`FUN_005c2060` posts it
/// with 1): the selection state's slot `+0xc` with Ctrl.
/// - An enemy ship, while ships or a fighter group of ours are selected:
///   a target added, or dropped when held (`0x005a1af0`, `0x005a24d0`,
///   `0x005a3aa0`, `0x005a4bd0`). With no such selection it starts a new
///   one (`0x005a0d10`, `0x005a1240`).
/// - One of our ships, with a fighter group selected (`0x005a4bd0`): its
///   escort dropped when the group escorts it (`FUN_005ca310`, then order
///   0), else the group escorts it.
/// - One of our ships, with a task force selected (`0x005a3aa0`): the
///   force's escort of it dropped; else the ship joins the force
///   (`FUN_005ade50`) or, as a member, leaves it (`FUN_005a8d30`), and the
///   selection ends with its last member.
/// - Otherwise it joins or leaves the selection (`0x005a1af0`,
///   `0x005a24d0`).
fn ctrl_click_ship(session: &mut BattleSession, index: usize) -> &'static str {
    let Some(ship) = session.ships.get(index) else {
        return "none";
    };
    let player_is_attacker = session.player_is_attacker;
    let enemy = ship.is_attacker != player_is_attacker;
    let fighter_group = session.selected_fighter_group;
    let own_ships_selected = session
        .ships
        .iter()
        .any(|ship| ship.alive && ship.selected && ship.is_attacker == player_is_attacker);
    if enemy {
        let target = TacticalAttackTarget::CapitalShip(index);
        if fighter_group.is_some() {
            assign_selected_fighter_group_target(session, target, true);
            return "fighter_target";
        }
        if own_ships_selected {
            assign_selected_focus_target(session, target, true);
            return "target";
        }
        select_clicked_ship(session, index, true);
        return "selected";
    }
    if let Some(group) = fighter_group {
        let members = (0..session.fighters.len())
            .filter(|&each| {
                let fighter = &session.fighters[each];
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.is_attacker == player_is_attacker
                    && fighter.fighter_group == group
            })
            .collect::<Vec<_>>();
        let held = members
            .iter()
            .any(|&each| session.fighters[each].escort_target == Some(index));
        if held {
            session.leave_escort(&[], &members);
            for each in members {
                let fighter = &mut session.fighters[each];
                fighter.escort_target = None;
                fighter.order = TacticalOrder::None;
            }
            return "escort_dropped";
        }
        assign_selected_escort(session, index);
        return "escort";
    }
    if let Some(group) = session.task_force_selection {
        let escorted = session.ships.iter().any(|ship| {
            ship.alive && ship.task_force == group && ship.escort_target == Some(index)
        });
        if escorted {
            let escorts = (0..session.ships.len())
                .filter(|&each| {
                    let ship = &session.ships[each];
                    ship.task_force == group && ship.escort_target == Some(index)
                })
                .collect::<Vec<_>>();
            session.leave_escort(&escorts, &[]);
            for each in escorts {
                let ship = &mut session.ships[each];
                ship.escort_target = None;
                ship.order = TacticalOrder::None;
            }
            return "escort_dropped";
        }
        if session.ships[index].task_force == group {
            let ship = &mut session.ships[index];
            ship.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
            ship.selected = false;
            let left = session.ships.iter().any(|ship| {
                ship.alive && ship.is_attacker == player_is_attacker && ship.task_force == group
            });
            if !left {
                session.task_force_selection = None;
                session.selected_ship = None;
            } else if session.selected_ship == Some(index) {
                session.selected_ship = session.ships.iter().position(|ship| {
                    ship.alive && ship.selected && ship.is_attacker == player_is_attacker
                });
            }
            return "left_task_force";
        }
        let ship = &mut session.ships[index];
        ship.task_force = group;
        ship.selected = true;
        return "joined_task_force";
    }
    select_clicked_ship(session, index, true);
    "toggled"
}

/// `assign_selected_capital_focus_target` for a capital ship or a squadron.
fn assign_selected_focus_target(
    session: &mut BattleSession,
    target: TacticalAttackTarget,
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
    // The one-unit state's target (`0x005a1af0`) first gives the unit a
    // force of its own (`FUN_005aded0`), out of its numbered group: in the
    // A0 VM run of 2026-10-27 a right click on the enemy took "Victory
    // Destroyer 1" from "Task Force # 1" to "Task Force Unassigned".
    if selected.len() == 1 && session.task_force_selection.is_none() {
        form_selected_task_force(session, &selected);
    }
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
    // The task-force state (`0x66bc48`): orders go to the whole force.
    session.task_force_selection = Some(group);
    // Its slot `+0x3c` (`FUN_005a42e0`) hums the selected unit.
    session.selection_state_open = true;
    session.ships.iter().filter(|ship| ship.selected).count()
}

/// A left click on a capital ship. The clicked unit's side picks the
/// selection state (`0x005a0d10`, `0x005a1240`): the player's own ship gets
/// the one-unit state (`0x66bce8`, `FUN_005a1550`), or with Ctrl joins the
/// several-unit state (`0x66bc98`); an enemy ship gets the enemy state
/// (`0x66bba8`, `FUN_005a1110`). That state shows the ship's panel like the
/// one-unit state, but its order and tactic slots (`+0x20`, `+0x24`,
/// `+0x28`) are empty, and a Ctrl click from it starts a new one-unit
/// selection instead of adding to it.
fn select_clicked_ship(session: &mut BattleSession, index: usize, control: bool) {
    let Some(ship) = session.ships.get(index) else {
        return;
    };
    let own = ship.is_attacker == session.player_is_attacker;
    let player_is_attacker = session.player_is_attacker;
    let joins = control
        && own
        && session
            .ships
            .iter()
            .all(|ship| !ship.selected || ship.is_attacker == player_is_attacker);
    if joins {
        session.ships[index].selected = !session.ships[index].selected;
    } else {
        for ship in &mut session.ships {
            ship.selected = false;
        }
        session.ships[index].selected = true;
    }
    session.selected_ship = Some(index);
    session.selected_fighter_group = None;
    session.task_force_selection = None;
    // The one-unit and enemy states hum their unit (`FUN_005a1770`,
    // `FUN_005a1110`); the several-unit state has no slot `+0x3c` sound.
    session.selection_state_open = !joins;
    for fighter in &mut session.fighters {
        fighter.selected = false;
    }
}

/// A click reaches a navigation point only when no unit is under the
/// pointer (port: the units come first, for want of a pick depth).
fn no_unit_under_pointer(
    enemy_ship: Option<usize>,
    friendly_ship: Option<usize>,
    enemy_squadron: Option<usize>,
) -> bool {
    enemy_ship.is_none() && friendly_ship.is_none() && enemy_squadron.is_none()
}

/// Select a navigation point: the unit selection ends (`FUN_00595b70(0,
/// 0)`, the empty state) and the point becomes the selection (`0x66bb58`).
fn select_navigation_point(
    session: &mut BattleSession,
    selected_navigation_point: &mut Option<TacticalWorldPosition>,
    point: TacticalWorldPosition,
) {
    for ship in &mut session.ships {
        ship.selected = false;
    }
    for fighter in &mut session.fighters {
        fighter.selected = false;
    }
    session.selected_ship = None;
    session.selected_fighter_group = None;
    session.task_force_selection = None;
    session.selection_state_open = false;
    *selected_navigation_point = Some(point);
}

fn assign_selected_to_task_force(session: &mut BattleSession, group: u8) -> usize {
    let mut assigned_groups = [false; 8];
    for ship in &session.ships {
        if ship.alive && ship.is_attacker == session.player_is_attacker && ship.task_force < 8 {
            assigned_groups[usize::from(ship.task_force)] = true;
        }
    }
    let next_blank = assigned_groups
        .iter()
        .position(|assigned| !assigned)
        .and_then(|index| u8::try_from(index).ok());
    if !assigned_groups[usize::from(group.min(7))] && next_blank != Some(group.min(7)) {
        return 0;
    }
    if !assigned_groups[usize::from(group.min(7))] {
        // A new force starts Stand Off (`0x005cbf50`).
        session.task_force_formations[usize::from(session.player_is_attacker)]
            [usize::from(group.min(7))] = TacticalTactic::StandOff;
    }
    let mut assigned = 0;
    for ship in &mut session.ships {
        if ship.alive && ship.selected && ship.is_attacker == session.player_is_attacker {
            ship.task_force = group.min(7);
            assigned += 1;
        }
    }
    if assigned > 0 {
        BattleSession::refresh_original_task_force_geometry(
            &mut session.ships,
            &session.task_force_formations,
        );
    }
    assigned
}

fn select_fighter_group(session: &mut BattleSession, group: u8) -> bool {
    // `FUN_005ad530` takes every fighter group of the withdrawing player's
    // side off the command panel (`FUN_005a0320`, the flags `+0x154` that
    // `FUN_0059f5f0` asks).
    if session.withdrawal_ms.is_some() {
        return false;
    }
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
    session.task_force_selection = None;
    // hyp: the group state is `0x66bbf8`, whose slot `+0x3c`
    // (`FUN_005a5230`) hums the group's unit (`FUN_005c6070`).
    session.selection_state_open = true;
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
    // A one-unit state for the next ship (`FUN_005a1770`).
    session.selection_state_open = true;
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
    let escorting_capitals = (0..session.ships.len())
        .filter(|&index| {
            let ship = &session.ships[index];
            index != target
                && ship.alive
                && !ship.retreating
                && ship.selected
                && ship.is_attacker == target_side
        })
        .collect::<Vec<_>>();
    let escorting_fighters = (0..session.fighters.len())
        .filter(|&index| {
            let fighter = &session.fighters[index];
            fighter.alive
                && fighter.squad_count > 0
                && fighter.selected
                && fighter.is_attacker == target_side
        })
        .collect::<Vec<_>>();
    session.leave_escort(&escorting_capitals, &escorting_fighters);

    for &index in &escorting_capitals {
        let ship = &mut session.ships[index];
        ship.order = TacticalOrder::Escort;
        ship.escort_target = Some(target);
        ship.attack_target = None;
        ship.manual_targets.clear();
        ship.navigation_route.clear();
    }
    for &index in &escorting_fighters {
        let fighter = &mut session.fighters[index];
        fighter.order = TacticalOrder::Escort;
        fighter.escort_target = Some(target);
        fighter.attack_target = None;
        fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
        fighter.recovery_target = None;
    }
    let (capitals, fighters) = (escorting_capitals.len(), escorting_fighters.len());
    (capitals, fighters)
}

/// `FUN_005ad530`'s task-force loop for the player's fighter groups
/// (`BattleSession::withdraw_fighters`).
pub fn withdraw_player_fighters(session: &mut BattleSession) {
    session.withdraw_fighters(session.player_is_attacker);
}

/// The units' states before an order or tactic:
/// - one unit (`0x005a1a60`, `0x005a1ac0`) takes a force of its own
///   (`FUN_005aded0`) outside the group slots;
/// - several (`0x005a2c70`, `0x005a2a40`) form one new force
///   (`0x005ade70`) in the first free group slot (`0x0059f610`), found before
///   they leave their old ones, and the task-force state follows
///   (`0x005a33d0`);
/// - with no free slot, each takes a force of its own.
fn form_selected_task_force(session: &mut BattleSession, members: &[usize]) {
    let force = match members {
        [] => return,
        [_] => TACTICAL_UNGROUPED_TASK_FORCE,
        _ => (0_u8..8)
            .find(|slot| {
                !session.ships.iter().any(|ship| {
                    ship.alive
                        && ship.is_attacker == session.player_is_attacker
                        && ship.task_force == *slot
                })
            })
            .unwrap_or(TACTICAL_UNGROUPED_TASK_FORCE),
    };
    for &index in members {
        session.ships[index].task_force = force;
    }
    if force != TACTICAL_UNGROUPED_TASK_FORCE {
        // A new force starts Stand Off (`0x005cbf50` builds `0x005ec310`).
        session.task_force_formations[usize::from(session.player_is_attacker)]
            [usize::from(force)] = TacticalTactic::StandOff;
        session.task_force_selection = Some(force);
        BattleSession::refresh_original_task_force_geometry(
            &mut session.ships,
            &session.task_force_formations,
        );
    }
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
        let ordered = (0..session.fighters.len())
            .filter(|&index| {
                let fighter = &session.fighters[index];
                fighter.alive && fighter.selected && fighter.is_attacker == session.player_is_attacker
            })
            .collect::<Vec<_>>();
        session.leave_escort(&[], &ordered);
        for &index in &ordered {
            let fighter = &mut session.fighters[index];
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
    } else {
        // The input state's order and tactic slots (`+0x20`, `+0x28`): a
        // chosen task force takes them itself (`0x005a3630`, `0x005a3740`);
        // selected units first become forces (`form_selected_task_force`).
        let player_is_attacker = session.player_is_attacker;
        let members = session
            .ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| {
                ship.alive
                    && !ship.retreating
                    && ship.is_attacker == player_is_attacker
                    && session
                        .task_force_selection
                        .map_or(ship.selected, |group| ship.task_force == group)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if session.task_force_selection.is_none() {
            form_selected_task_force(session, &members);
        }
        session.leave_escort(&members, &[]);
        for &index in &members {
            let ship = &mut session.ships[index];
            ship.order = order;
            ship.escort_target = None;
            if let Some(tactic) = tactic {
                ship.tactic = tactic;
            }
            selected_capitals.push(index);
            capital_members += 1;
        }
        // SetTactic (`0x005c9be0`) stores `+0x38` at once and posts `0x3d`;
        // its turn rebuilds the force's formation. A force of its own holds
        // one ship, whose formation has no other member.
        if let (Some(tactic), Some(group)) = (tactic, session.task_force_selection) {
            if !members.is_empty() {
                let stamp = session.command_event_stamp();
                session.post_turn_event(
                    stamp,
                    TacticalTurnEvent::TaskForceTactic {
                        attacker: player_is_attacker,
                        task_force: group,
                        tactic,
                    },
                );
            }
        }
    }
    // `SetOrder` (`0x005ca530`) sets the order at once and posts its
    // executor (event `0xa`) for the command stamp.
    if !selected_capitals.is_empty() || !selected_fighters.is_empty() {
        session.post_order_executor(order, selected_capitals, selected_fighters);
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
        let group = match session.task_force_selection {
            Some(group) => group,
            None => {
                let groups = session
                    .ships
                    .iter()
                    .filter(|ship| {
                        ship.alive
                            && ship.selected
                            && ship.is_attacker == session.player_is_attacker
                    })
                    .map(|ship| ship.task_force)
                    .collect::<Vec<_>>();
                match groups.as_slice() {
                    [] => return,
                    // `0x005a1a60`: one unit's order, the generic
                    // assignment line (`0x77`, `0xf5`).
                    [TACTICAL_UNGROUPED_TASK_FORCE] => {
                        if panel != TacticalCommandPanel::Display {
                            session
                                .pending_voice_cues
                                .push(TacticalVoiceCue::capital_mission(
                                    faction,
                                    TACTICAL_UNGROUPED_TASK_FORCE,
                                ));
                        }
                        return;
                    }
                    // `0x005a2c70` with no free slot (`0x84`, `0x102`).
                    [TACTICAL_UNGROUPED_TASK_FORCE, ..] => {
                        session
                            .pending_voice_cues
                            .push(TacticalVoiceCue::target_task_force_rejected(faction));
                        return;
                    }
                    [group, ..] => *group,
                }
            }
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
    gates: BattleOptionsGates,
) -> Option<TacticalBattleOptionsControl> {
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    let (mouse_x, mouse_y) = mouse_position();
    let (x, y) = canvas.logical_pointer(mouse_x, mouse_y);
    tactical_battle_options_control_at(cache, gates, x, y)
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
    // A cut never reaches past the bitmap.
    let source_rect = source_rect.map(|rect| Rect {
        w: rect.w.min(texture.width() - rect.x),
        h: rect.h.min(texture.height() - rect.y),
        ..rect
    });
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
    // The battle windows' texts raster at their own sizes in both faces
    // (`battle_window_text_params`); key them apart from the others.
    let window_sizes = [
        BATTLE_WINDOW_FONT_B,
        BATTLE_WINDOW_FONT_6,
        BATTLE_WINDOW_FONT_4,
        BATTLE_RESULT_NAME_FONT,
    ]
    .map(|em| battle_window_text_params(canvas, em, WHITE).font_size);
    let window_key = |size: u16| u16::MAX / 2 + size;
    if sizes.iter().all(|size| warmed_sizes.contains(size))
        && warmed_sizes.contains(&(u16::MAX - tactical_text_font_size(canvas)))
        && window_sizes
            .iter()
            .all(|&size| warmed_sizes.contains(&window_key(size)))
    {
        return;
    }

    let mut characters: std::collections::BTreeSet<_> =
        Font::latin_character_list().into_iter().collect();
    for text in [
        "0123456789x# :",
        "Task Force No Orders RETREAT Battle Paused Observe Battle",
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
    // The panel font keeps its own atlas; key its size apart from the
    // default font's.
    let panel_size = tactical_text_font_size(canvas);
    if warmed_sizes.insert(u16::MAX - panel_size) {
        measure_text(
            &sample,
            tactical_arial_bold().as_ref(),
            panel_size,
            BATTLE_WINDOW_FONT_6 / TACTICAL_FONT_RASTER * TACTICAL_STATUS_ASPECT,
        );
        for em in [TACTICAL_FONT_8_EM, TACTICAL_FONT_10_EM, TACTICAL_STATUS_EM] {
            measure_text(
                &sample,
                tactical_arial().as_ref(),
                panel_size,
                em / TACTICAL_FONT_RASTER,
            );
        }
    }
    for size in window_sizes {
        if warmed_sizes.insert(window_key(size)) {
            for font in [tactical_arial(), tactical_arial_bold()] {
                measure_text(&sample, font.as_ref(), size, 1.0);
            }
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
            draw_tactical_bitmap(
                cache,
                art::MANEUVERS_PANEL,
                canvas,
                TACTICAL_PANEL_X,
                TACTICAL_PANEL_Y,
            );
            if let Some(graphic) = pending_order.maneuver_graphic() {
                draw_tactical_bitmap(cache, graphic, canvas, 496.0, 42.0);
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
            draw_tactical_bitmap(
                cache,
                art::MISSIONS_PANEL,
                canvas,
                TACTICAL_PANEL_X,
                TACTICAL_PANEL_Y,
            );
            draw_tactical_bitmap(
                cache,
                pending_order
                    .mission_graphic(player_is_alliance)
                    .unwrap_or(art::MISSIONS_HUD_EMPTY),
                canvas,
                496.0,
                42.0,
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

    draw_tactical_bitmap(
        cache,
        art::WITHDRAW_CONFIRMATION_PANEL,
        canvas,
        TACTICAL_PANEL_X,
        TACTICAL_PANEL_Y,
    );
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
    // at (14, 92) in the 149x236 panel with font 10 (`FUN_00601c60`), pure
    // white. A0 TAC-06-C003 shows their capitals from rows 59, 123 and 137,
    // at x 502 and 496.
    for (text, x, cap_top) in [
        ("Withdraw Confirmation", 502.0, 59.0),
        ("Are you sure you wish to", 496.0, 123.0),
        ("order a fleet withdraw?", 496.0, 137.0),
    ] {
        draw_tactical_panel_text(canvas, text, x, cap_top, TACTICAL_FONT_10);
    }
}

/// A main-HUD control's tooltip (TEXTTACT, `FUN_0059a8d0`'s builders) and
/// whether it is enabled: task forces 0xad93 + i, or 0xad92 "Unassigned
/// Task Force" while empty (`FUN_0059b550`, `FUN_0059b450`); fighter groups
/// 0xad9d + i, disabled while empty (`FUN_0059b690`); nav point sets
/// 0xada2 + i; Pause 0xadb7, 0xadb8 "Resume Battle" while paused; the
/// highlights 0xada6 / 0xada8, "Dim" 0xada7 / 0xada9 while lit
/// (`FUN_0059b130`); the camera controls 0xadb6..0xadab; Game Options 0xadaa.
fn hud_tooltip(
    control: TacticalHudControl,
    task_forces: [bool; 8],
    fighter_groups: [bool; 4],
    paused: bool,
    highlight_alliance: bool,
    highlight_empire: bool,
) -> (u16, bool) {
    match control {
        TacticalHudControl::TaskForce(i) => {
            let assigned = task_forces[usize::from(i)];
            (
                if assigned {
                    0xad93 + u16::from(i)
                } else {
                    0xad92
                },
                true,
            )
        }
        TacticalHudControl::FighterGroup(i) => {
            (0xad9d + u16::from(i), fighter_groups[usize::from(i)])
        }
        TacticalHudControl::NavigationSet(i) => (0xada2 + u16::from(i), true),
        TacticalHudControl::Pause => (if paused { 0xadb8 } else { 0xadb7 }, true),
        TacticalHudControl::ZoomIn => (0xadb6, true),
        TacticalHudControl::ZoomOut => (0xadb5, true),
        TacticalHudControl::CameraLeft => (0xadb4, true),
        TacticalHudControl::CameraRight => (0xadb3, true),
        TacticalHudControl::CameraUp => (0xadb2, true),
        TacticalHudControl::CameraDown => (0xadb1, true),
        TacticalHudControl::CameraTarget => (0xadb0, true),
        TacticalHudControl::CameraRecall => (0xadad, true),
        TacticalHudControl::CameraMemorize => (0xadab, true),
        TacticalHudControl::HighlightAlliance => {
            (if highlight_alliance { 0xada7 } else { 0xada6 }, true)
        }
        TacticalHudControl::HighlightEmpire => {
            (if highlight_empire { 0xada9 } else { 0xada8 }, true)
        }
        TacticalHudControl::BattleOptions => (0xadaa, true),
    }
}

/// A side panel's control tooltip (TEXTTACT): the panel decides the
/// shared confirm button's text (Assign Maneuver / Tactics 0xddb9, Assign
/// Mission 0xdda0); Surround / Stand Off names the other tactic
/// (`FUN_005dd080`); Previous / Next name the task force when the ship has
/// one (9f 0xdd81 / 0xdd82, 9g 0xdd84 / 0xdd85).
fn command_tooltip(
    control: TacticalCommandControl,
    maneuvers: bool,
    surround: bool,
    in_task_force: bool,
) -> u16 {
    match control {
        TacticalCommandControl::PreviousCapital => {
            if in_task_force {
                0xdd81
            } else {
                0xdd84
            }
        }
        TacticalCommandControl::NextCapital => {
            if in_task_force {
                0xdd82
            } else {
                0xdd85
            }
        }
        TacticalCommandControl::OpenManeuvers => 0xddcd,
        TacticalCommandControl::OpenMissions => 0xdd8c,
        TacticalCommandControl::LeftHook => 0xddd0,
        TacticalCommandControl::RightHook => 0xddd1,
        TacticalCommandControl::Hammer => 0xddd2,
        TacticalCommandControl::Anvil => 0xddd3,
        TacticalCommandControl::HoldPosition => 0xddd4,
        TacticalCommandControl::ToggleTactic => {
            if surround {
                0xddce
            } else {
                0xddcf
            }
        }
        TacticalCommandControl::AttackCapitalShips => 0xdde5,
        TacticalCommandControl::AttackFighters => 0xddeb,
        TacticalCommandControl::Recover => 0xdde2,
        TacticalCommandControl::AttackDeathStar => 0xdde6,
        TacticalCommandControl::Confirm => {
            if maneuvers {
                0xddb9
            } else {
                0xdda0
            }
        }
        TacticalCommandControl::Cancel => 0xdda1,
    }
}

/// Register the tooltip of the battle control under the pointer, for the
/// controls `draw_original_tactical_hud` shows this frame
/// (`ghidra/notes/tooltips.md` §9).
#[expect(
    clippy::too_many_arguments,
    reason = "The HUD's state comes as the drawing's own parameters."
)]
fn register_tactical_tooltips(
    canvas: TacticalCanvas,
    groups: &TacticalGroupHud<'_>,
    attack_death_star_enabled: bool,
    paused: bool,
    (highlight_alliance, highlight_empire): (bool, bool),
    command_panel: TacticalCommandPanel,
    battle_options_open: bool,
    withdraw_confirmation_open: bool,
    player_observing: bool,
    gates: BattleOptionsGates,
) {
    use crate::win_tooltip::StringModule::Tactical;
    // The headers (FUN_0059be80, FUN_0059c030).
    tactical_tool(
        canvas,
        NativeRect::new(9.0, 1.0, 49.0, 19.0),
        0x9a00,
        Tactical,
        0xad91,
        true,
    );
    tactical_tool(
        canvas,
        NativeRect::new(296.0, 1.0, 49.0, 19.0),
        0x9a01,
        Tactical,
        0xad9b,
        true,
    );
    for (index, rect) in TASK_FORCE_BUTTONS.into_iter().enumerate() {
        let control = TacticalHudControl::TaskForce(index as u8);
        let (string, enabled) = hud_tooltip(
            control,
            groups.task_forces,
            groups.fighter_groups,
            paused,
            highlight_alliance,
            highlight_empire,
        );
        tactical_tool(
            canvas,
            rect,
            0x9a10 | index as u64,
            Tactical,
            string,
            enabled,
        );
    }
    for (index, rect) in FIGHTER_GROUP_BUTTONS.into_iter().enumerate() {
        let control = TacticalHudControl::FighterGroup(index as u8);
        let (string, enabled) = hud_tooltip(
            control,
            groups.task_forces,
            groups.fighter_groups,
            paused,
            highlight_alliance,
            highlight_empire,
        );
        // FUN_00602150 makes the buttons 25 by 15.
        let rect = NativeRect::new(rect.x, rect.y, rect.width, 15.0);
        tactical_tool(
            canvas,
            rect,
            0x9a20 | index as u64,
            Tactical,
            string,
            enabled,
        );
    }
    for spec in TACTICAL_HUD_CONTROLS {
        let (string, enabled) = hud_tooltip(
            spec.control,
            groups.task_forces,
            groups.fighter_groups,
            paused,
            highlight_alliance,
            highlight_empire,
        );
        let id = 0x9a40 | u64::from(string);
        tactical_tool(canvas, spec.rect, id, Tactical, string, enabled);
    }
    if withdraw_confirmation_open {
        // FUN_005f0f40: Confirm Withdraw 0xddc2, Cancel 0xdda1.
        for (spec, string) in TACTICAL_WITHDRAW_CONFIRMATION_CONTROLS
            .into_iter()
            .zip([0xddc2, 0xdda1])
        {
            tactical_tool(
                canvas,
                spec.rect,
                0x9b00 | u64::from(string),
                Tactical,
                string,
                true,
            );
        }
        return;
    }
    if battle_options_open {
        // FUN_005dab90; Observe / Take Command names the other state.
        for spec in TACTICAL_BATTLE_OPTIONS_CONTROLS {
            let (string, enabled) = match spec.control {
                TacticalBattleOptionsControl::Withdraw => (0xddba, gates.withdraw),
                TacticalBattleOptionsControl::SimulateRemainder => (0xddbb, gates.simulate),
                TacticalBattleOptionsControl::ObserveToggle => (
                    if player_observing { 0xdded } else { 0xddbc },
                    gates.observe,
                ),
                TacticalBattleOptionsControl::GameOptions => (0xddbd, true),
                TacticalBattleOptionsControl::Close => (0xddbe, true),
            };
            tactical_tool(
                canvas,
                spec.rect,
                0x9c00 | u64::from(string),
                Tactical,
                string,
                enabled,
            );
        }
        return;
    }
    let in_task_force = groups.selected_ship_group.is_some();
    let command =
        |control: TacticalCommandControl, maneuvers: bool, surround: bool, enabled: bool| {
            let string = command_tooltip(control, maneuvers, surround, in_task_force);
            let id = 0x9d00 | (u64::from(maneuvers) << 16) | u64::from(string);
            tactical_tool(
                canvas,
                tactical_command_spec(control, true).0,
                id,
                Tactical,
                string,
                enabled,
            );
        };
    match command_panel {
        TacticalCommandPanel::Maneuvers { pending_tactic, .. } => {
            let surround = pending_tactic == TacticalTactic::Surround;
            for control in [
                TacticalCommandControl::LeftHook,
                TacticalCommandControl::RightHook,
                TacticalCommandControl::Hammer,
                TacticalCommandControl::Anvil,
                TacticalCommandControl::HoldPosition,
                TacticalCommandControl::ToggleTactic,
                TacticalCommandControl::Confirm,
                TacticalCommandControl::Cancel,
            ] {
                command(control, true, surround, true);
            }
        }
        TacticalCommandPanel::Missions { .. } => {
            let fighter = groups.selected_fighter_group.is_some();
            for (control, enabled) in [
                (TacticalCommandControl::AttackCapitalShips, true),
                (TacticalCommandControl::AttackFighters, true),
                (TacticalCommandControl::Recover, fighter),
                (
                    TacticalCommandControl::AttackDeathStar,
                    attack_death_star_enabled,
                ),
                (TacticalCommandControl::Confirm, true),
                (TacticalCommandControl::Cancel, true),
            ] {
                command(control, false, false, enabled);
            }
        }
        TacticalCommandPanel::Display => {
            let has_selection =
                groups.selected_fighter_group.is_some() || groups.selected_ship_name.is_some();
            if !has_selection {
                return;
            }
            if groups.selected_ship_name.is_some() {
                command(TacticalCommandControl::PreviousCapital, false, false, true);
                command(TacticalCommandControl::NextCapital, false, false, true);
                // FUN_005e4110: the hull and shield bars, then the power
                // allocation rect tools 0xddd5..0xddd9.
                if groups.selected_ship_hull.is_some() {
                    tactical_tool(
                        canvas,
                        TACTICAL_SELECTED_HULL_METER,
                        0x9e00,
                        Tactical,
                        0xdd9a,
                        true,
                    );
                }
                if groups.selected_ship_shield.is_some() {
                    tactical_tool(
                        canvas,
                        TACTICAL_SELECTED_SHIELD_METER,
                        0x9e01,
                        Tactical,
                        0xdd9b,
                        true,
                    );
                }
                if groups.selected_ship_subsystems.is_some() {
                    for (index, x) in TACTICAL_SUBSYSTEM_X.into_iter().enumerate() {
                        tactical_tool(
                            canvas,
                            NativeRect::new(x, TACTICAL_SUBSYSTEM_Y, 22.0, 15.0),
                            0x9e10 | index as u64,
                            Tactical,
                            0xddd5 + index as u16,
                            true,
                        );
                    }
                }
            }
            if !groups.selected_enemy {
                command(TacticalCommandControl::OpenManeuvers, false, false, true);
                command(TacticalCommandControl::OpenMissions, false, false, true);
            }
        }
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
    gates: BattleOptionsGates,
    pressed_options: Option<TacticalBattleOptionsControl>,
    pressed_withdraw_confirmation: Option<TacticalWithdrawConfirmationControl>,
    death_star: Option<TacticalDeathStar>,
    death_star_laser_pressed: bool,
    death_star_targeting: bool,
    contents_scroll: f32,
    battle_palette: Option<(u32, Vec<[u8; 3]>)>,
) {
    use resources::tactical as art;

    register_tactical_tooltips(
        canvas,
        &groups,
        attack_death_star_enabled,
        paused,
        (highlight_alliance, highlight_empire),
        command_panel,
        battle_options_open,
        withdraw_confirmation_open,
        player_observing,
        gates,
    );
    if let Some(death_star) = death_star {
        draw_tactical_bitmap(
            cache,
            art::DEATH_STAR_LASER_GAUGE,
            canvas,
            DEATH_STAR_LASER_GAUGE_RECT.x,
            DEATH_STAR_LASER_GAUGE_RECT.y,
        );
        let (x, y) = canvas.point(DEATH_STAR_LASER_FILL_RECT.x, DEATH_STAR_LASER_FILL_RECT.y);
        draw_rectangle(
            x,
            y,
            DEATH_STAR_LASER_FILL_RECT.width * canvas.scale,
            DEATH_STAR_LASER_FILL_RECT.height * canvas.scale,
            DEATH_STAR_LASER_EMPTY_COLOR,
        );
        let charged = death_star_laser_charge_width(death_star.laser_charge);
        if charged > 0.0 {
            draw_rectangle(
                x,
                y,
                charged * canvas.scale,
                DEATH_STAR_LASER_FILL_RECT.height * canvas.scale,
                DEATH_STAR_LASER_CHARGE_COLOR,
            );
        }
        let ready = death_star.operational()
            && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE;
        draw_tactical_bitmap(
            cache,
            death_star_laser_control_resource(
                ready,
                death_star_laser_pressed || death_star_targeting,
            ),
            canvas,
            DEATH_STAR_LASER_BUTTON_RECT.x,
            DEATH_STAR_LASER_BUTTON_RECT.y,
        );
    }

    // The top task-force and fighter-group strips occupy slots in bitmap 1000;
    // A0 TAC-02-C002 and TAC-01-C002 put the headers at x 9 and 296.
    draw_tactical_bitmap(
        cache,
        if player_is_alliance {
            art::TASK_FORCES_ALLIANCE
        } else {
            art::TASK_FORCES_EMPIRE
        },
        canvas,
        9.0,
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
            TASK_FORCE_BUTTONS[usize::from(index)].x,
            2.0,
        );
        if assigned {
            draw_task_force_number(canvas, index, active);
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
        296.0,
        1.0,
    );
    for index in 0..4_u8 {
        let assigned = groups.fighter_groups[usize::from(index)];
        let active = assigned
            && (groups.selected_fighter_group == Some(index)
                || pressed_control == Some(TacticalHudControl::FighterGroup(index)));
        let button = FIGHTER_GROUP_BUTTONS[usize::from(index)];
        draw_original_bitmap(
            cache,
            DllSource::Tactical,
            if active {
                FIGHTER_GROUP_PRESSED_ART[usize::from(index)]
            } else if assigned {
                FIGHTER_GROUP_NORMAL_ART[usize::from(index)]
            } else {
                art::SQUADRON_UNASSIGNED
            },
            canvas,
            button.x,
            button.y,
            Some(Rect::new(0.0, 0.0, button.width, button.height)),
        );
    }

    let has_selection =
        groups.selected_fighter_group.is_some() || groups.selected_ship_name.is_some();
    if withdraw_confirmation_open {
        draw_tactical_withdraw_confirmation(cache, canvas, pressed_withdraw_confirmation);
    } else if battle_options_open {
        draw_tactical_bitmap(
            cache,
            art::BATTLE_OPTIONS_PANEL,
            canvas,
            TACTICAL_PANEL_X,
            TACTICAL_PANEL_Y,
        );
        for spec in TACTICAL_BATTLE_OPTIONS_CONTROLS {
            if !gates.enabled(spec.control) {
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
        let panel_resource = tactical_display_panel_resource(
            groups.selected_fighter_group.is_some(),
            groups.selected_ship_name.is_some(),
            player_is_alliance,
        );
        draw_tactical_bitmap(
            cache,
            panel_resource,
            canvas,
            TACTICAL_PANEL_X,
            TACTICAL_PANEL_Y,
        );
    }
    if !withdraw_confirmation_open
        && !battle_options_open
        && command_panel == TacticalCommandPanel::Display
    {
        if let Some(name) = groups.panel_ship_name() {
            if let Some(resource) = groups.selected_ship_hud {
                draw_tactical_bitmap(
                    cache,
                    resource,
                    canvas,
                    TACTICAL_SELECTED_SHIP_ART_X,
                    TACTICAL_SELECTED_SHIP_ART_Y,
                );
            }
            if let Some((hull, maximum)) = groups.selected_ship_hull {
                draw_tactical_selected_meter(
                    canvas,
                    TACTICAL_SELECTED_HULL_METER,
                    tactical_meter_fraction(hull, maximum),
                );
            }
            if let Some((shield, maximum)) = groups.selected_ship_shield {
                draw_tactical_selected_meter(
                    canvas,
                    TACTICAL_SELECTED_SHIELD_METER,
                    tactical_meter_fraction(shield, maximum),
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
            draw_tactical_contents(
                cache,
                canvas,
                groups.selected_ship_contents,
                contents_scroll,
                battle_palette.as_ref(),
            );
            // A0 TAC-03-C016: capitals from row 37 at x 508, the caption
            // from row 82, the orders from rows 212 and 223 at x 497.
            draw_tactical_panel_text(canvas, name, 508.0, 37.0, TACTICAL_FONT_8);
            draw_tactical_panel_text(
                canvas,
                &groups.panel_caption(),
                508.0,
                82.0,
                TACTICAL_FONT_8,
            );
            draw_tactical_panel_text(
                canvas,
                &groups.panel_orders_line(),
                497.0,
                212.0,
                TACTICAL_FONT_8,
            );
            // FUN_005e4b40 `+0xb4`: the tactic only for our unit.
            if !groups.selected_enemy {
                draw_tactical_panel_text(
                    canvas,
                    &format!("Tactics: {}", groups.selected_tactic.label()),
                    497.0,
                    223.0,
                    TACTICAL_FONT_8,
                );
            }
        } else if let (Some(_), Some(group), Some(hud_resource)) = (
            groups.selected_fighter_name,
            groups.selected_fighter_group,
            groups.selected_fighter_hud,
        ) {
            // A0 TAC-02-C002/C003: the portrait (2030, 2031) at (505, 87);
            // the group, its orders and its tactics from rows 39, 50 and
            // 61 at x 508, 503 and 502.
            draw_tactical_bitmap(cache, hud_resource, canvas, 505.0, 87.0);
            let labels = ["Red Group", "Blue Group", "Green Group", "Gold Group"];
            draw_tactical_panel_text(
                canvas,
                labels[usize::from(group.min(3))],
                508.0,
                39.0,
                TACTICAL_FONT_8,
            );
            draw_tactical_panel_text(
                canvas,
                &groups.fighter_orders_line(),
                503.0,
                50.0,
                TACTICAL_FONT_8,
            );
            draw_tactical_panel_text(
                canvas,
                &format!("Tactics: {}", groups.selected_tactic.label()),
                502.0,
                61.0,
                TACTICAL_FONT_8,
            );
            // Label `+0xb0` moved to panel (0x17, 0x92): capitals from row
            // 176 at x 505 (A0 TAC-02-C002 "X-wings: 1 12/12").
            if let Some(count) = groups.selected_fighter_count {
                draw_tactical_panel_text(canvas, &count.line(), 505.0, 176.0, TACTICAL_FONT_8);
            }
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
        // FUN_005e4b40: buttons 0xdd01 and 0xdd02 hide for a unit that
        // takes no orders (FUN_00596830), an enemy's.
        let order_buttons: &[(TacticalCommandControl, u32, u32)] = if groups.selected_enemy {
            &[]
        } else {
            &[
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
            ]
        };
        for &(control, normal, pressed_art) in order_buttons {
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

    for (index, x) in [485.0, 524.0, 563.0, 602.0].into_iter().enumerate() {
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
        483.0,
        305.0,
    );
    draw_tactical_bitmap(
        cache,
        if highlight_alliance {
            art::HIGHLIGHT_ALLIANCE_SHIPS
        } else {
            art::DIM_ALLIANCE_SHIPS
        },
        canvas,
        515.0,
        305.0,
    );
    draw_tactical_bitmap(
        cache,
        if paused {
            art::BTN_PAUSE_PAUSED
        } else {
            art::BTN_PAUSE_RUNNING
        },
        canvas,
        561.0,
        308.0,
    );
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::BattleOptions) {
            art::BTN_BATTLE_OPTIONS_PRESSED
        } else {
            art::BTN_BATTLE_OPTIONS_NORMAL
        },
        canvas,
        608.0,
        309.0,
    );

    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::ZoomIn) {
            art::BTN_CAMERA_ZOOM_IN_PRESSED
        } else {
            art::BTN_CAMERA_ZOOM_IN_NORMAL
        },
        canvas,
        491.0,
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
        607.0,
        343.0,
    );
    for (id, x, y) in [
        (
            if pressed_control == Some(TacticalHudControl::CameraLeft) {
                art::BTN_CAMERA_LEFT_PRESSED
            } else {
                art::BTN_CAMERA_LEFT_NORMAL
            },
            497.0,
            381.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraRight) {
                art::BTN_CAMERA_RIGHT_PRESSED
            } else {
                art::BTN_CAMERA_RIGHT_NORMAL
            },
            582.0,
            381.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraUp) {
                art::BTN_CAMERA_UP_PRESSED
            } else {
                art::BTN_CAMERA_UP_NORMAL
            },
            539.0,
            338.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraDown) {
                art::BTN_CAMERA_DOWN_PRESSED
            } else {
                art::BTN_CAMERA_DOWN_NORMAL
            },
            540.0,
            424.0,
        ),
        (
            if pressed_control == Some(TacticalHudControl::CameraTarget) {
                art::BTN_CAMERA_TARGET_PRESSED
            } else {
                art::BTN_CAMERA_TARGET_NORMAL
            },
            549.0,
            391.0,
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
        486.0,
        444.0,
    );
    draw_tactical_bitmap(
        cache,
        if pressed_control == Some(TacticalHudControl::CameraMemorize) {
            art::BTN_CAMERA_MEMORIZE_PRESSED
        } else {
            art::BTN_CAMERA_MEMORIZE_NORMAL
        },
        canvas,
        594.0,
        444.0,
    );
}

/// A maneuver button (`0xdd10`..`0xdd14`) has style 8 (`FUN_006030c0(8)`),
/// so each click flips its pressed state (`FUN_00604170`); the others are
/// released (`FUN_005dd140`). The commit (`FUN_005dd3a0`) takes the pressed
/// one, or order 0 with none: clicking the chosen maneuver again cancels it,
/// as the manual's "deselect any selected orders and confirm" does.
fn toggled_maneuver(pending: TacticalOrder, clicked: TacticalOrder) -> TacticalOrder {
    if pending == clicked {
        TacticalOrder::None
    } else {
        clicked
    }
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
                *pending_order = toggled_maneuver(*pending_order, TacticalOrder::LeftHook);
            }
        }
        TacticalCommandControl::RightHook => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = toggled_maneuver(*pending_order, TacticalOrder::RightHook);
            }
        }
        TacticalCommandControl::Hammer => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = toggled_maneuver(*pending_order, TacticalOrder::Hammer);
            }
        }
        TacticalCommandControl::Anvil => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = toggled_maneuver(*pending_order, TacticalOrder::Anvil);
            }
        }
        TacticalCommandControl::HoldPosition => {
            if let TacticalCommandPanel::Maneuvers { pending_order, .. } = &mut state.command_panel
            {
                *pending_order = toggled_maneuver(*pending_order, TacticalOrder::HoldPosition);
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
            if let Some(session) = state.session.as_mut() {
                session.set_player_observing(state.player_observing);
            }
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
        state.asset_renderer.clear_chase();
        return false;
    };
    let Some(session) = state.session.as_ref() else {
        state.camera_follow = None;
        state.asset_renderer.clear_chase();
        return false;
    };
    // F1-F8: the chase camera behind the task force's lead hull
    // (`FUN_005aafc0` gives the group's object, `FUN_005c2990` mode 0xb
    // builds class `0x66d478`, `0x005d9b10` places it). port: the lead is
    // the group's first live hull, and the group's second object (vt
    // `+0x44`) is not traced, so the camera takes the no-target branch.
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
    // F1-F8: the chase camera behind the task force's lead hull
    // (`FUN_005aafc0` gives the group's object, `FUN_005c2990` mode 0xb
    // builds class `0x66d478`, `0x005d9b10` places it). port: the lead is
    // the group's first live hull, and the group's second object (vt
    // `+0x44`) is not traced, so the camera takes the no-target branch.
    // F9-F12 keep the look-at below: a fighter sprite has no mesh box.
    let chase = match follow {
        TacticalCameraFollow::TaskForce(group) => session
            .ships
            .iter()
            .find(|ship| {
                ship.alive
                    && !ship.retreating
                    && ship.is_attacker == session.player_is_attacker
                    && ship.task_force == group
            })
            .map(|lead| {
                (
                    lead.tactical_resource,
                    lead.source_position,
                    lead.source_forward,
                )
            }),
        TacticalCameraFollow::FighterGroup(_) => None,
    };
    if source_positions.is_empty() {
        state.camera_follow = None;
        state.asset_renderer.clear_chase();
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
    let Some((resource, position, forward)) = chase else {
        state.asset_renderer.clear_chase();
        state
            .asset_renderer
            .focus_target(object_id, source.rendered());
        return true;
    };
    let bounds = resource
        .and_then(|resource| {
            state
                .asset_renderer
                .participant_box(resource.mesh_resource_base)
        })
        .unwrap_or([0.0; 6]);
    let view = crate::tactical_assets::original_chase_view(
        object_id,
        macroquad::math::vec3(position.x, position.y, position.z),
        macroquad::math::vec3(forward.x, forward.y, forward.z),
        bounds,
    );
    state.asset_renderer.set_chase(view);
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
        let gates = BattleOptionsGates::of(state.session.as_ref());
        let hovered = tactical_battle_options_control_at(cache, gates, x, y);
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
    let pressed = is_mouse_button_pressed(MouseButton::Left);
    let released = is_mouse_button_released(MouseButton::Left);
    if !pressed && !released && !is_mouse_button_down(MouseButton::Left) {
        state.command_control_pressed = None;
    }
    if pressed || (released && state.command_control_pressed.is_some()) {
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
        // A side-panel button acts on its release over it (the original's
        // buttons notify on WM_LBUTTONUP); the HUD's act on the press.
        if pressed && state.command_panel == TacticalCommandPanel::Display {
            if let Some(session) = state.session.as_ref() {
                let hud = TacticalGroupHud::from_session(session);
                let count = hud.selected_ship_contents.len();
                let scroll = contents_scroll_of(state.contents_scroll, hud.selected_ship_index);
                let arrow = [(CONTENTS_LEFT_ARROW, false), (CONTENTS_RIGHT_ARROW, true)]
                    .into_iter()
                    .find(|(rect, _)| rect.contains(x, y))
                    .map(|(_, right)| right);
                if let (Some(right), Some(_)) = (arrow, contents_thumb(count, scroll)) {
                    state.contents_scroll = (
                        hud.selected_ship_index,
                        scrolled_contents(count, scroll, right),
                    );
                    return TacticalAction::None;
                }
            }
        }
        if pressed {
            if let Some(control) = command_control {
                state.command_control_pressed = Some(control);
            } else if let Some(control) = tactical_hud_control_at(cache, x, y) {
                if let Some(action) = activate_tactical_hud_control(state, control) {
                    return action;
                }
            }
        }
        if released {
            if let Some(captured) = state
                .command_control_pressed
                .take()
                .filter(|captured| Some(*captured) == command_control)
            {
                activate_tactical_command(state, captured);
            }
        }
    }
    // FUN_0059b730: a key presses its HUD button (`FUN_0059ca90`). Keys 1
    // to 8 first release the group buttons (`FUN_0059b430`, `FUN_0059b600`).
    // port: a held key acts once, at its press.
    for (key, control) in TACTICAL_BUTTON_KEYS {
        if is_key_pressed(key) {
            if let Some(action) = activate_tactical_hud_control(state, control) {
                return action;
            }
        }
    }
    // F1 to F8 and F9 to F12: the camera follows that task force or fighter
    // group when it exists (`FUN_00595ba0`, `FUN_00595bc0`, then
    // `FUN_005c2990` mode 0xb). The selection stays.
    let follow_keys = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
    ];
    for (index, key) in follow_keys.iter().enumerate() {
        if !is_key_pressed(*key) {
            continue;
        }
        let Some(session) = state.session.as_ref() else {
            continue;
        };
        if let Some(follow) = camera_follow_for_key(session, index) {
            state.camera_follow = Some(follow);
            let (kind, group) = match follow {
                TacticalCameraFollow::TaskForce(group) => ("task_force", group),
                TacticalCameraFollow::FighterGroup(group) => ("fighter_group", group),
            };
            macroquad::logging::info!(
                "[tactical_camera] command=follow kind={} group={}",
                kind,
                group + 1,
            );
        }
    }
    // Tab goes to the panel window (`FUN_0059b730` case 9, `+0x10c`); the
    // capital panel (`0x66d6e0`, `0x005e45a0`) presses its next button
    // `0xdd04` on Tab and its previous one `0xdd03` on Shift+Tab.
    if is_key_pressed(KeyCode::Tab) {
        if let Some(control) = tab_panel_control(state, is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)) {
            activate_tactical_command(state, control);
        }
    }
    if is_key_pressed(KeyCode::Space) {
        if let Some(session) = state.session.as_mut() {
            if let Some(TacticalFocusAssignment::Assigned { object_ids, .. }) = attack_nearest_enemy(session) {
                macroquad::logging::info!(
                    "[tactical_navigation] event=nearest_target members={}",
                    object_ids.len(),
                );
            }
        }
    }
    TacticalAction::None
}

/// `FUN_0059b730`'s keys and the buttons they press: Enter and NumPad. the
/// camera target (`0xad21`), Page Up, NumPad 9 and NumPad + zoom in
/// (`0xad1b`), Page Down, NumPad 3 and NumPad - zoom out (`0xad1c`), End
/// and NumPad 1 recall (`0xad1a`), Home and NumPad 7 memorize (`0xad19`),
/// the arrows and NumPad 4, 8, 6, 2 turn the camera (`0xad1d`, `0xad1f`,
/// `0xad1e`, `0xad20`), and 1 to 8 the task forces (`0xad02`..`0xad09`).
const TACTICAL_BUTTON_KEYS: [(KeyCode, TacticalHudControl); 28] = [
    (KeyCode::Enter, TacticalHudControl::CameraTarget),
    (KeyCode::KpDecimal, TacticalHudControl::CameraTarget),
    (KeyCode::PageUp, TacticalHudControl::ZoomIn),
    (KeyCode::Kp9, TacticalHudControl::ZoomIn),
    (KeyCode::KpAdd, TacticalHudControl::ZoomIn),
    (KeyCode::PageDown, TacticalHudControl::ZoomOut),
    (KeyCode::Kp3, TacticalHudControl::ZoomOut),
    (KeyCode::KpSubtract, TacticalHudControl::ZoomOut),
    (KeyCode::End, TacticalHudControl::CameraRecall),
    (KeyCode::Kp1, TacticalHudControl::CameraRecall),
    (KeyCode::Home, TacticalHudControl::CameraMemorize),
    (KeyCode::Kp7, TacticalHudControl::CameraMemorize),
    (KeyCode::Left, TacticalHudControl::CameraLeft),
    (KeyCode::Kp4, TacticalHudControl::CameraLeft),
    (KeyCode::Up, TacticalHudControl::CameraUp),
    (KeyCode::Kp8, TacticalHudControl::CameraUp),
    (KeyCode::Right, TacticalHudControl::CameraRight),
    (KeyCode::Kp6, TacticalHudControl::CameraRight),
    (KeyCode::Down, TacticalHudControl::CameraDown),
    (KeyCode::Kp2, TacticalHudControl::CameraDown),
    (KeyCode::Key1, TacticalHudControl::TaskForce(0)),
    (KeyCode::Key2, TacticalHudControl::TaskForce(1)),
    (KeyCode::Key3, TacticalHudControl::TaskForce(2)),
    (KeyCode::Key4, TacticalHudControl::TaskForce(3)),
    (KeyCode::Key5, TacticalHudControl::TaskForce(4)),
    (KeyCode::Key6, TacticalHudControl::TaskForce(5)),
    (KeyCode::Key7, TacticalHudControl::TaskForce(6)),
    (KeyCode::Key8, TacticalHudControl::TaskForce(7)),
];

/// F1 + `index`: F1 to F8 follow task forces 1 to 8, F9 to F12 fighter
/// groups 1 to 4 (`FUN_0059b730`, cases `0x70`..`0x7b`).
fn camera_follow_for_key(session: &BattleSession, index: usize) -> Option<TacticalCameraFollow> {
    let group = u8::try_from(index % 8).ok()?;
    if index < 8 {
        task_force_exists(session, group).then_some(TacticalCameraFollow::TaskForce(group))
    } else {
        fighter_group_exists(session, group).then_some(TacticalCameraFollow::FighterGroup(group))
    }
}

/// The capital panel's button Tab presses, when that panel is shown. port:
/// the squadron panel (`0x66d750`, `0x005e74e0`) has the same Tab keys,
/// but the port's squadron panel has no previous and next buttons.
fn tab_panel_control(state: &TacticalState, shift: bool) -> Option<TacticalCommandControl> {
    let session = state.session.as_ref()?;
    let shown = state.command_panel == TacticalCommandPanel::Display
        && !state.battle_options_open
        && !state.withdraw_confirmation_open
        && TacticalGroupHud::from_session(session).panel_ship_name().is_some();
    shown.then_some(if shift {
        TacticalCommandControl::PreviousCapital
    } else {
        TacticalCommandControl::NextCapital
    })
}

/// A player's task force with a member left. port: `FUN_00595ba0`'s list
/// of the occupied slots.
fn task_force_exists(session: &BattleSession, group: u8) -> bool {
    session.ships.iter().any(|ship| {
        ship.alive && ship.is_attacker == session.player_is_attacker && ship.task_force == group
    })
}

/// A player's fighter group with a squadron left. port: `FUN_00595bc0`'s
/// list of the occupied groups.
fn fighter_group_exists(session: &BattleSession, group: u8) -> bool {
    session.fighters.iter().any(|fighter| {
        fighter.alive
            && fighter.squad_count > 0
            && fighter.is_attacker == session.player_is_attacker
            && fighter.fighter_group == group
    })
}

/// A HUD button (`FUN_0059b130`), pressed by the mouse or by its key
/// (`FUN_0059b730`).
fn activate_tactical_hud_control(
    state: &mut TacticalState,
    control: TacticalHudControl,
) -> Option<TacticalAction> {
    match control {
        TacticalHudControl::NavigationSet(set) => {
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
        TacticalHudControl::TaskForce(group) => {
            state.selected_navigation_point = None;
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
        TacticalHudControl::FighterGroup(group) => {
            state.selected_navigation_point = None;
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
        TacticalHudControl::Pause => return Some(TacticalAction::TogglePause),
        TacticalHudControl::ZoomIn => {
            state.camera_follow = None;
            state.zoom = (state.zoom * 1.25).min(2.0);
            state.asset_renderer.zoom_in();
        }
        TacticalHudControl::ZoomOut => {
            state.camera_follow = None;
            state.zoom = (state.zoom / 1.25).max(0.5);
            state.asset_renderer.zoom_out();
        }
        TacticalHudControl::CameraTarget => {
            state.camera_follow = None;
            // The point state's slot +4 (0x005a55f0) gives the point.
            let selected = state.session.as_ref().and_then(camera_target_of_selection).or_else(|| {
                state
                    .selected_navigation_point
                    .map(|point| (TACTICAL_NAVIGATION_CAMERA_ID, 0.0, 0.0, point))
            });
            if let Some((object_id, x, y, source_position)) = selected {
                if object_id != TACTICAL_NAVIGATION_CAMERA_ID {
                    (state.camera_x, state.camera_y) = camera_offset_for_target(x, y);
                }
                state
                    .asset_renderer
                    .focus_target(object_id, source_position.rendered());
            } else {
                state.asset_renderer.clear_target();
            }
        }
        TacticalHudControl::CameraLeft => {
            if state.camera_follow.take().is_some() {
                macroquad::logging::info!(
                    "[tactical_camera] command=follow_exit reason=manual"
                );
            }
            state.asset_renderer.turn_left();
        }
        TacticalHudControl::CameraRight => {
            state.camera_follow = None;
            state.asset_renderer.turn_right();
        }
        TacticalHudControl::CameraUp => {
            state.camera_follow = None;
            state.asset_renderer.pitch_up();
        }
        TacticalHudControl::CameraDown => {
            state.camera_follow = None;
            state.asset_renderer.pitch_down();
        }
        TacticalHudControl::CameraMemorize => {
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
        TacticalHudControl::CameraRecall => {
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
        TacticalHudControl::HighlightAlliance => {
            state.highlight_alliance = !state.highlight_alliance;
        }
        TacticalHudControl::HighlightEmpire => {
            state.highlight_empire = !state.highlight_empire;
        }
        TacticalHudControl::BattleOptions => {
            state.battle_options_open = true;
            state.withdraw_confirmation_open = false;
            state.withdraw_confirmation_pressed = None;
            state.command_panel = TacticalCommandPanel::Display;
            macroquad::logging::info!("[tactical_options] command=open");
        }
    }
    None
}

/// The camera target button (`0xad21`, `FUN_005c2990` mode 9): the free
/// camera looks at the selection state's unit (its slot `+4` through
/// `FUN_00599c80`). A ship selection gives its ship (`0x005a5910`,
/// `0x005a5680`); a fighter group its current squadron, the first that
/// still flies (`0x005a4710`, `FUN_005c6070`). Squadron objects count from
/// 1001, as the fighter projections do.
fn camera_target_of_selection(
    session: &BattleSession,
) -> Option<(u32, f32, f32, TacticalWorldPosition)> {
    if let Some(group) = session.selected_fighter_group {
        let ours = |fighter: &&TacticalFighter| {
            fighter.squad_count > 0
                && fighter.is_attacker == session.player_is_attacker
                && fighter.fighter_group == group
        };
        if let Some((index, fighter)) = session
            .fighters
            .iter()
            .enumerate()
            .find(|(_, fighter)| fighter.alive && ours(fighter))
        {
            let object_id = u32::try_from(index).ok()?.checked_add(1001)?;
            return Some((object_id, fighter.x, fighter.y, fighter.source_position));
        }
        // A group whose squadrons are all inside a hangar: the squadron's
        // frame rides with its carrier, so the camera looks at the carrier
        // (A0 VM 2026-10-27, Battle at Yavin: the Red group's camera target
        // shows its Victory Destroyer before any launch).
        let carrier = session
            .fighters
            .iter()
            .filter(ours)
            .find(|fighter| fighter.in_hangar)
            .and_then(|fighter| fighter.home_carrier)?;
        let ship = session.ships.get(carrier).filter(|ship| ship.alive)?;
        let object_id = u32::try_from(carrier).ok()?.checked_add(1)?;
        return Some((object_id, ship.x, ship.y, ship.source_position));
    }
    let index = session.selected_ship?;
    let ship = session.ships.get(index)?;
    let object_id = u32::try_from(index).ok()?.checked_add(1)?;
    Some((object_id, ship.x, ship.y, ship.source_position))
}

/// The free camera's frame step (`Class_0066d418` slot `+0xc`, `0x005d9640`)
/// looks at its target object's frame each frame (`FUN_005c1080`, then the
/// frame's `LookAt`, `+0x90`), so the view keeps up with a moving target.
/// The target stays through turns and zooms (`0x005d97c0` changes only
/// angles and the field) and comes back with a recalled view (`+0x44` is
/// part of the state, `0x005d9900`).
fn track_tactical_camera_target(state: &mut TacticalState) {
    let Some(object_id) = state.asset_renderer.target_object_id() else {
        return;
    };
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Some(position) = tracked_target_position(session, object_id) else {
        return;
    };
    // The drawn point: the step's pose blended as the scene draws it.
    let previous = state
        .previous_poses
        .as_ref()
        .and_then(|poses| poses.before_latest_step(session))
        .and_then(|poses| match usize::try_from(object_id).ok()? {
            index @ 1001..10_001 => poses.fighters.get(index - 1001).copied(),
            index => poses.ships.get(index.checked_sub(1)?).map(|(position, _)| *position),
        });
    let (drawn, _) = presented_pose(
        previous.map(|from| (from, TacticalWorldVector::SOURCE_FORWARD)),
        (position, TacticalWorldVector::SOURCE_FORWARD),
        session.step_accumulator,
    );
    state.asset_renderer.focus_target(object_id, drawn.rendered());
}

/// The camera's object id for a selected navigation point; it does not move,
/// so it is not tracked.
const TACTICAL_NAVIGATION_CAMERA_ID: u32 = 30_001;

/// The current position of a camera target object: ships count from 1,
/// squadrons from 1001 (`camera_target_of_selection`). port: a lost unit
/// keeps its last point.
fn tracked_target_position(session: &BattleSession, object_id: u32) -> Option<TacticalWorldPosition> {
    let index = usize::try_from(object_id).ok()?;
    if (1001..10_001).contains(&index) {
        let fighter = session.fighters.get(index - 1001)?;
        return fighter.alive.then_some(fighter.source_position);
    }
    let ship = session.ships.get(index.checked_sub(1)?)?;
    (index < 1001 && ship.alive).then_some(ship.source_position)
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
    /// The damage mark between them (`FUN_0042c3b0`, `+0x50` bit `0x200`):
    /// a craft's GOKRES portrait + `0x2000`.
    mark: Option<(DllSource, u32)>,
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
    /// The paler scene under the list tabs (surface `0x33`).
    list_scene: u32,
    rail: u32,
    tabs: [[u32; 2]; 4],
    tab_rects: [NativeRect; 4],
    buttons: [[u32; 3]; 3],
}

/// `FUN_0044f860` builds surface `0x32` on the scene `0x29d8`/`0x29d9` and
/// surface `0x33` on the paler `0x29da`/`0x29db`; `FUN_00450660` shows
/// `0x32` on the summary tab and `0x33` on the three list tabs.
fn battle_alert_scene(skin: &BattleAlertSkin, tab: BattleAlertTab) -> u32 {
    if BattleAlertList::of(tab).is_some() {
        skin.list_scene
    } else {
        skin.scene
    }
}

/// The Battle Alert's bitmaps in paint order. The tab's scene fills the
/// frame's 400-wide aperture at (12, 13); the list tabs' 410-wide scene is
/// cut there (A0 alert-imperial-forces). The frame, keyed on blue, lies
/// over the scene: in A0 TAC-01-C010 (window (62, 49), list scene 10714)
/// 5931 of the frame's pixels inside the aperture show the frame and none
/// shows the scene alone.
fn battle_alert_layers(
    skin: &BattleAlertSkin,
    tab: BattleAlertTab,
) -> [(u32, f32, f32, Option<Rect>); 2] {
    [
        (
            battle_alert_scene(skin, tab),
            12.0,
            13.0,
            Some(Rect::new(0.0, 0.0, 400.0, 310.0)),
        ),
        (skin.frame, 0.0, 0.0, None),
    ]
}

fn battle_alert_skin(player_is_alliance: bool) -> BattleAlertSkin {
    use resources::strategy as art;
    if player_is_alliance {
        BattleAlertSkin {
            frame: art::BATTLE_ALERT_WINDOW_ALLIANCE,
            scene: art::BATTLE_ALERT_SCENE_ALLIANCE,
            list_scene: art::BATTLE_ALERT_SCENE_ALLIANCE_DISABLED,
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
            list_scene: art::BATTLE_ALERT_SCENE_EMPIRE_DISABLED,
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

/// A key the Battle Summary's own key slot answers (`FUN_004710f0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleSummaryKey {
    /// `0x1b`: slot `+0x30` closes the window, then `FUN_0041ce80` ends the
    /// battle's done choice, as the Close button `0xc9` does.
    Close,
    /// `0x25` (left) or `0x27` (right): the category bar's neighbour.
    Step { right: bool },
}

/// The summary's key slot (`FUN_004710f0`): Escape, then the arrows. Every
/// other key goes to the Command Center's `WM_KEYDOWN` (`+0x20` slot
/// `+0x14`), where `FUN_00422ce0` drops F1..F7 while a modal window is up.
fn battle_summary_key(escape: bool, left: bool, right: bool) -> Option<BattleSummaryKey> {
    if escape {
        Some(BattleSummaryKey::Close)
    } else if left || right {
        Some(BattleSummaryKey::Step { right })
    } else {
        None
    }
}

/// The category an arrow selects (`FUN_004710f0` on the bar `+0x160`): the
/// nearest visible button before (left) or after (right) the selected one;
/// none when every one of them is hidden (the bar shows on the forces pages
/// only); the first button when the selected one has no neighbour that way.
fn battle_category_after_arrow(
    current: BattleResultCategory,
    right: bool,
    bar_shown: bool,
) -> Option<BattleResultCategory> {
    let all = BattleResultCategory::ALL;
    let index = all.iter().position(|category| *category == current).unwrap_or(0);
    let neighbour = if right {
        all.get(index + 1)
    } else {
        index.checked_sub(1).and_then(|before| all.get(before))
    };
    match neighbour {
        Some(category) => bar_shown.then_some(*category),
        None => Some(all[0]),
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
    // The damaged object's GOKRES portrait + 0x2000, under the portrait
    // (FUN_0042c3b0). A lost unit wears it too: its full damage keeps the
    // bit, as the user saw on the original (2026-10-28).
    let mark = |picture: Option<(DllSource, u32)>, damaged: bool| {
        picture
            .filter(|_| damaged)
            .map(|(source, id)| (source, id + 0x2000))
    };
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
                .map(|ship| {
                    let picture = portrait(
                        world
                            .capital_ship_classes
                            .get(ship.class_key)
                            .and_then(|class| crate::minis::capital_ship_mini_id(class.dat_id)),
                    );
                    // FUN_00501c00: hull damage or a system hit.
                    let damaged = ship.hull_current < ship.hull_max
                        || ship.subsystem_damage.total_hits() != 0;
                    // A ship whose jump ended has left (FUN_005a90c0, state
                    // 9), not been destroyed (state 0xb).
                    let lost = ship.hull_current <= 0 || (!ship.alive && !ship.retreated);
                    BattleResultEntry {
                        name: ship.name.clone(),
                        condition: condition(lost),
                        picture,
                        backdrop: None,
                        mark: mark(picture, damaged || lost),
                    }
                })
                .collect::<Vec<_>>();
            if let Some(death_star) = session
                .death_star
                .filter(|value| value.is_alliance == alliance)
            {
                let lost = death_star.destroyed || death_star.hull <= 0.0;
                let picture = portrait(crate::minis::capital_ship_mini_id(
                    rebellion_core::ids::DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
                ));
                entries.push(BattleResultEntry {
                    name: "Death Star".into(),
                    condition: condition(lost),
                    picture,
                    backdrop: None,
                    // A lost Death Star wears the damage mark as a lost ship
                    // does: A0 2026-11-18-edited-saves-7 d-15 (3c02b66f3450),
                    // the Imperial Forces tab after the trench run.
                    mark: mark(picture, lost),
                });
            }
            entries
        }
        BattleResultCategory::Fighters => session
            .fighters
            .iter()
            .filter(|fighter| fighter.identity.is_alliance == alliance)
            .map(|fighter| {
                let picture = portrait(
                    world
                        .fighter_classes
                        .get(fighter.class_key)
                        .and_then(|class| crate::minis::fighter_mini_id(class.dat_id)),
                );
                // FUN_00503340: its losses (slot +0x254) are not 0.
                let damaged = fighter.hull_current < fighter.hull_max;
                // A group whose jump ended has left (FUN_005a90c0); one in
                // its hangar (state 1) or docked (5) is aboard, and lost
                // only with its carrier.
                let docked = fighter.recovery_state
                    == rebellion_core::tactical::TacticalFighterRecoveryState::Recovered;
                let aboard = (fighter.in_hangar || docked).then(|| {
                    if docked {
                        fighter.recovery_target.or(fighter.home_carrier)
                    } else {
                        fighter.home_carrier
                    }
                });
                let lost = fighter.squad_count == 0
                    || match aboard {
                        Some(carrier) => carrier
                            .and_then(|carrier| session.ships.get(carrier))
                            .is_some_and(|ship| {
                                ship.hull_current <= 0 || (!ship.alive && !ship.retreated)
                            }),
                        None => !fighter.alive && !fighter.jumping(),
                    };
                BattleResultEntry {
                    name: fighter.name.clone(),
                    condition: condition(lost),
                    picture,
                    backdrop: None,
                    mark: mark(picture, damaged || lost),
                }
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
                        mark: None,
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
                    picture: rebellion_core::pictures::character_picture(character)
                    .map(|mini| (DllSource::Gokres, mini)),
                    backdrop: Some((DllSource::Strategy, 11_500)),
                    // FUN_004f7800: an injured character is damaged; its
                    // mini takes STRATEGY 11502 (FUN_0042c3b0).
                    mark: (character.injury > 0).then(|| {
                        crate::fleet_window::damage_mark(
                            crate::fleet_window::MiniObject::Character,
                            0,
                        )
                    }),
                })
                .collect()
        }
    }
}

/// The battle windows' texts in Arial (Liberation Sans): `size` is the em,
/// narrowed like the status texts (`TACTICAL_STATUS_ASPECT`). Their fonts
/// are the original's `0xb` (24 px cell, em 22) and 6 (18 px cell, em
/// 16.5): A0 TAC-01-C010 and TAC-07-C001 show 15- and 12-row capitals and
/// widths that fit those ems.
const BATTLE_WINDOW_FONT_B: f32 = 22.0;
const BATTLE_WINDOW_FONT_6: f32 = 16.5;
/// Font 4 (16 px cell): A0 TAC-07-C005 shows 10-row capitals; "Capital
/// Ships" spans 76 px. port: fitted em.
const BATTLE_WINDOW_FONT_4: f32 = 13.75;
/// The Battle Summary's unit names: "Medium Transport 2" spans 93 px in A0
/// TAC-07-C005. port: fitted em; the font index is not read.
const BATTLE_RESULT_NAME_FONT: f32 = 11.0;
/// Font 6's line step, its 18 px cell (A0 TAC-01-C010 rows 268 and 286).
const BATTLE_WINDOW_FONT_6_LINE: f32 = 18.0;

fn battle_window_text_params(
    canvas: TacticalCanvas,
    size: f32,
    color: Color,
) -> TextParams<'static> {
    // Rastered at its own size: a 10 px raster scaled up blurred the
    // windows' texts and dimmed their pure colour (A0 TAC-07-C012 draws
    // the destination page's lines in solid (255, 0, 0)).
    TextParams {
        font: None,
        font_size: (size * canvas.scale).round().max(1.0) as u16,
        font_scale: 1.0,
        font_scale_aspect: TACTICAL_STATUS_ASPECT,
        color,
        ..Default::default()
    }
}

/// Screen width of a battle-window text.
fn battle_window_text_width_in(canvas: TacticalCanvas, text: &str, size: f32, bold: bool) -> f32 {
    let font = if bold { tactical_arial_bold() } else { tactical_arial() };
    let params = battle_window_text_params(canvas, size, WHITE);
    measure_text(
        text,
        font.as_ref(),
        params.font_size,
        params.font_scale * params.font_scale_aspect,
    )
    .width
}

fn draw_result_text_in(
    canvas: TacticalCanvas,
    text: &str,
    x: f32,
    baseline_y: f32,
    (size, bold): (f32, bool),
    color: Color,
) {
    let font = if bold { tactical_arial_bold() } else { tactical_arial() };
    let (screen_x, screen_y) = canvas.point(x, baseline_y);
    draw_text_ex(
        text,
        screen_x,
        screen_y,
        TextParams {
            font: font.as_ref(),
            ..battle_window_text_params(canvas, size, color)
        },
    );
}

fn draw_result_wrapped_lines(
    canvas: TacticalCanvas,
    lines: &[String],
    x: f32,
    mut baseline_y: f32,
    max_width: f32,
    (size, bold): (f32, bool),
    color: Color,
) {
    // A line steps by its font's cell: 18 px for font 6.
    let line_step = (size * BATTLE_WINDOW_FONT_6_LINE / BATTLE_WINDOW_FONT_6).round();
    for line in lines {
        let mut current = String::new();
        for word in line.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if !current.is_empty()
                && battle_window_text_width_in(canvas, &candidate, size, bold) > max_width * canvas.scale
            {
                draw_result_text_in(canvas, &current, x, baseline_y, (size, bold), color);
                baseline_y += line_step;
                current = word.to_owned();
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            draw_result_text_in(canvas, &current, x, baseline_y, (size, bold), color);
            baseline_y += line_step;
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
    battle_report_lines(
        &loaded_battle_texts(),
        &battle_report(session, world, system_side),
    )
}

/// The report's fields for the session's viewer; `system_side` is the
/// system's side after the battle.
fn battle_report<'a>(
    session: &'a BattleSession,
    world: &'a GameWorld,
    system_side: Option<bool>,
) -> BattleReport<'a> {
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
    let viewer_alliance = player_is_alliance(session);
    // +0x48 (FUN_0049b720): where the viewer's withdrawn fleet went; the
    // report names no destination for the enemy's.
    let destination = (withdrawn == Some(viewer_alliance))
        .then(|| {
            rebellion_core::abstract_battle::withdrawal_destination(
                world,
                session.system,
                viewer_alliance,
            )
        })
        .flatten()
        .and_then(|key| world.systems.get(key))
        .map(|system| system.name.as_str());
    BattleReport {
        system_name: &session.system_name,
        viewer_alliance,
        winner_alliance: winner_is_alliance(session),
        withdrawn,
        destination,
        system_side,
        last_arrival: battle_alert_sides(world, session.system).1,
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
    /// `+0x48`: the viewer's withdrawn fleet's destination.
    destination: Option<&'a str>,
    /// The system's side bits after the battle; `None` neutral.
    system_side: Option<bool>,
    /// `+0x5c`: the system's last arrived side (`+0x78` bits 6..7, which
    /// `FUN_0049c0d0` copies to the battle's `+0x4c` at its start).
    last_arrival: Option<bool>,
}

/// The battle report's headline and body (`FUN_0049b0f0`): no winner
/// (`FUN_0049b150`), the viewer's win (`FUN_0049b330`) or loss
/// (`FUN_0049b4d0`), each sentence a TEXTSTRA RCDATA record appended to the
/// body. The viewer's withdrawn fleet "has withdrawn to {2:1}" (`0x700d`,
/// its destination `+0x48`); the enemy's "has withdrawn" (`0x700c`). port:
/// with no destination found the viewer's reads `0x700c` too (the
/// original writes no body). When the system's side arrived last (`+0x5c`)
/// the winner "maintained" (`0x7015`) or "broke" (`0x7016`) the blockade.
/// Missing texts fall back to the port's English.
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
    let viewer_withdrew = || match report.destination {
        Some(name) => say(0x700d, &[viewer, MessageParameter::Unit { name, class: "" }]),
        None => say(0x700c, &[viewer]),
    };
    let (headline, body): (Option<String>, Vec<Option<String>>) = match report.winner_alliance {
        None => {
            let ending = match report.withdrawn {
                None => vec![say(0x700f, &[viewer, enemy])],
                Some(side) if side == report.viewer_alliance => {
                    vec![viewer_withdrew(), say(0x700b, &[enemy])]
                }
                Some(_) => vec![say(0x700b, &[viewer]), say(0x700c, &[enemy])],
            };
            (
                say(0x7004, &[system]),
                std::iter::once(say(0x700e, &[viewer, enemy])).chain(ending).collect(),
            )
        }
        Some(winner) if winner == report.viewer_alliance => {
            // The side that arrived last held the system: it broke or
            // failed to break the blockade (FUN_0049b330).
            let first = match report.system_side {
                None => say(0x7007, &[system, enemy]),
                Some(side) if report.last_arrival == Some(side) => {
                    if side == report.viewer_alliance {
                        say(0x7016, &[system, viewer])
                    } else {
                        say(0x7015, &[system, viewer])
                    }
                }
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
            // FUN_0049b4d0: the enemy maintained or broke the blockade.
            let first = match report.system_side {
                None => say(0x700a, &[system, enemy]),
                Some(side) if report.last_arrival == Some(side) => {
                    if side == report.viewer_alliance {
                        say(0x7015, &[system, enemy])
                    } else {
                        say(0x7016, &[system, enemy])
                    }
                }
                Some(side) if side == report.viewer_alliance => say(0x7008, &[system, enemy]),
                Some(_) => say(0x7009, &[system, viewer]),
            };
            let ending = if report.withdrawn.is_some() {
                viewer_withdrew()
            } else {
                say(0x700b, &[viewer])
            };
            (say(0x7003, &[viewer]), vec![first, ending])
        }
    };
    match (headline, body.into_iter().collect::<Option<Vec<String>>>()) {
        // The headline keeps its record's trailing space (0x7002..0x7004
        // end ". " before the 0x01 end mark): FUN_0046f360 centres it in
        // the same 400-wide box as the title, and the space's advance
        // moves it half a space left (A0 TAC-01-C006, TAC-01-C013 and
        // empire-game3/battle-result-coruscant: x 71..344, centre 207.5,
        // where the title is centred on 212).
        (Some(headline), Some(body)) => vec![
            headline.trim_end_matches(|c: char| c.is_control()).to_owned(),
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
/// (`FUN_0049b150`, `FUN_0049b4d0`), " to {3:1}" the fleet's destination
/// (`+0x48`). port: with no destination found `0x7011` drops its
/// " to {3:1}", as the body reads `0x700c`.
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
        match report.destination {
            Some(name) => fill_message_text(
                template,
                &[viewer, system, MessageParameter::Unit { name, class: "" }],
            ),
            None => fill_message_text(&template.replace(" to {3:1}", ""), &[viewer, system]),
        }
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
    let mut ops = vec![
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: art::BATTLE_DESTINATION_BACKGROUND,
            x: BATTLE_RESULTS_SCENE_X,
            y: BATTLE_RESULTS_SCENE_Y,
        },
        // A0 TAC-07-C013: the Display page keeps the title, font 0xb from
        // window row 22.
        PageOp::Text {
            text: format!("Battle at {system_name}"),
            x: 212.0,
            baseline: 22.0 + 15.0,
            size: BATTLE_WINDOW_FONT_B,
            color: text_color,
            centered: true,
            bold: false,
        },
    ];
    // A0 TAC-07-C013: capitals from window rows 43 and 133 at x 24.
    // FUN_0046f360 sets font 7 (18 px cell, weight 700) for the second
    // line (`0x46fa33`), the first keeping font 6.
    for ((line, cap_top), bold) in lines.iter().zip([43.0, 133.0]).zip([false, true]) {
        ops.push(PageOp::Wrapped {
            lines: vec![line.clone()],
            x: 24.0,
            baseline: cap_top + 12.0,
            width: 328.0,
            size: BATTLE_WINDOW_FONT_6,
            color: text_color,
            bold,
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
            baseline: 281.0 + 12.0,
            size: BATTLE_WINDOW_FONT_6,
            color: WHITE,
            centered: true,
            bold: false,
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
        battle_summary_scene(&battle_report(session, world, None)),
        &battle_result_summary_lines(session, world),
        text_color,
    );
    let frame = battle_result_skin(player_is_alliance(session)).frame;
    draw_page_ops(cache, canvas, frame_over_page(ops, frame));
}

/// The summary page's scene, which `FUN_0046f360` picks for the viewer
/// from the report's winner (`+0x50`) and the side that withdrew
/// (`+0x54`): with no winner `0x2d63` (11619). Otherwise, for the
/// Alliance's viewer: a win `0x2a07` (10759), or `0x2a06` (10758) when the
/// Empire withdrew; its own withdrawal `0x2a05` (10757); a loss `0x2a08`
/// (10760). For the Empire's: a win `0x2a08`, or `0x2a05` when the
/// Alliance withdrew; its own withdrawal `0x2a06`; a loss `0x2a07`.
fn battle_summary_scene(report: &BattleReport<'_>) -> u32 {
    use resources::strategy as art;
    let Some(winner) = report.winner_alliance else {
        return art::EVENT_BATTLE_MUTUAL_DESTRUCTION;
    };
    let viewer = report.viewer_alliance;
    // 10757 / 10758: the Alliance's or the Empire's fleet beaten off.
    let beaten = |alliance: bool| {
        if alliance {
            art::EVENT_BATTLE_ALLIANCE_DEFEATED
        } else {
            art::EVENT_BATTLE_EMPIRE_DEFEATED
        }
    };
    // 10759 / 10760: the Alliance's or the Empire's victory.
    let victory = |alliance: bool| {
        if alliance {
            art::EVENT_BATTLE_ALLIANCE_VICTORY
        } else {
            art::EVENT_BATTLE_EMPIRE_VICTORY
        }
    };
    if winner == viewer {
        if report.withdrawn == Some(!viewer) {
            beaten(!viewer)
        } else {
            victory(viewer)
        }
    } else if report.withdrawn == Some(viewer) {
        beaten(viewer)
    } else {
        victory(!viewer)
    }
}

/// The Battle Summary's summary page (`FUN_0046f360`): the scene; the title
/// at (12, 18), 400 wide, centred, font `0xb`; the outcome's first line at
/// (12, 225), 400 wide, centred, font `0xb`; the rest at (24, 255), 352
/// wide, word-wrapped, font 6, over a black shadow at (25, 256). The text
/// takes the viewer's colour (red for side 1, green for side 2). The
/// capture guide-059 shows the same layout.
fn battle_summary_page(
    system_name: &str,
    scene: u32,
    lines: &[String],
    text_color: Color,
) -> Vec<PageOp> {
    let mut ops = vec![
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: scene,
            x: BATTLE_RESULTS_SCENE_X,
            y: BATTLE_RESULTS_SCENE_Y,
        },
        // A0 TAC-07-C001: title capitals from window row 22, the
        // headline's from 229 (font 0xb), the rest's from 258 (font 6),
        // FUN_0046f360's rects plus each font's internal leading.
        PageOp::Text {
            text: format!("Battle at {system_name}"),
            x: 212.0,
            baseline: 22.0 + 15.0,
            size: BATTLE_WINDOW_FONT_B,
            color: text_color,
            centered: true,
            bold: false,
        },
    ];
    if let Some((headline, rest)) = lines.split_first() {
        ops.push(PageOp::Text {
            text: headline.clone(),
            x: 212.0,
            baseline: 229.0 + 15.0,
            size: BATTLE_WINDOW_FONT_B,
            color: text_color,
            centered: true,
            bold: false,
        });
        for (x, cap_top, color) in [(25.0, 259.0, BLACK), (24.0, 258.0, text_color)] {
            ops.push(PageOp::Wrapped {
                lines: rest.to_vec(),
                x,
                baseline: cap_top + 12.0,
                width: 352.0,
                size: BATTLE_WINDOW_FONT_6,
                color,
                bold: false,
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
    scroll: usize,
    pressed: Option<crate::list_scroll_bar::Part>,
    player_alliance: bool,
    text_color: Color,
) {
    let entries = battle_result_entries(session, world, troop_transport, alliance, category);
    let mut ops = battle_force_page(
        &session.system_name,
        &entries,
        alliance,
        category,
        scroll,
        player_alliance,
        text_color,
        |source, id| cache.original_resource_size(source, id),
    );
    let content = battle_result_content(&entries, category);
    ops.extend(battle_result_bar_ops(&battle_result_bar(content, scroll), pressed));
    let frame = battle_result_skin(player_alliance).frame;
    draw_page_ops(cache, canvas, frame_over_page(ops, frame));
}

/// The forces list's content height in pixels: its longest column's rows.
fn battle_result_content(entries: &[BattleResultEntry], category: BattleResultCategory) -> f32 {
    battle_result_list_length(entries, category) as f32 * battle_result_row_height(category)
}

/// A forces list row's height (`FUN_004714e0` -> `FUN_006082c0`): 70, or
/// 50 for Personnel.
fn battle_result_row_height(category: BattleResultCategory) -> f32 {
    if category == BattleResultCategory::Personnel {
        50.0
    } else {
        70.0
    }
}

/// The forces list's scroll bar (`FUN_0046f360` at 0x4704be:
/// `FUN_0060a490(.., list, 7, 0x299d, ..)`) in window coordinates.
fn battle_result_bar(content: f32, scroll: usize) -> crate::list_scroll_bar::ScrollBar {
    crate::list_scroll_bar::ScrollBar::new(
        crate::list_scroll_bar::RESULTS_ART,
        37.0,
        136.0,
        347.0,
        BATTLE_RESULT_LIST_HEIGHT,
        content,
        scroll as f32,
    )
}

/// The bar's bitmaps (STRATEGY.DLL), clipped to the bar.
fn battle_result_bar_ops(
    bar: &crate::list_scroll_bar::ScrollBar,
    pressed: Option<crate::list_scroll_bar::Part>,
) -> Vec<PageOp> {
    let pieces = bar.pieces(pressed);
    if pieces.is_empty() {
        return Vec::new();
    }
    std::iter::once(PageOp::Clip(Some(bar.rect())))
        .chain(pieces.into_iter().map(|piece| PageOp::Bitmap {
            source: DllSource::Strategy,
            id: piece.id,
            x: piece.x,
            y: piece.y,
        }))
        .chain(std::iter::once(PageOp::Clip(None)))
        .collect()
}

/// The forces list's rows: its longest column.
fn battle_result_list_length(
    entries: &[BattleResultEntry],
    category: BattleResultCategory,
) -> usize {
    battle_result_columns(entries, category == BattleResultCategory::Personnel)
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
}

/// A live hull, or a destroyed one whose fireball has not yet reached
/// `FUN_005d41a0`'s call at 11, which deletes the hull's mesh.
fn hull_in_scene(session: &BattleSession, index: usize) -> bool {
    session.ships.get(index).is_some_and(|ship| ship.alive)
        || session.impact_effects.iter().any(|effect| {
            effect.target == index
                && effect.sequence == OriginalTacticalEffectSequence::Destroyed
                && effect.hull_shown()
        })
}

/// The forces list's first pixel after the wheel turns `wheel` with the
/// pointer at `pointer` (logical coordinates): only over the list at (37,
/// 136) in the window, 347 by 166. port: the original has no wheel; a
/// notch moves a row, within the bar's range.
fn result_list_scroll(
    scroll: usize,
    pointer: (f32, f32),
    wheel: f32,
    content: f32,
    category: BattleResultCategory,
) -> usize {
    let (x, y) = (
        pointer.0 - battle_window_origin().0,
        pointer.1 - battle_window_origin().1,
    );
    let over = (37.0..37.0 + 347.0).contains(&x)
        && (136.0..136.0 + BATTLE_RESULT_LIST_HEIGHT).contains(&y);
    if !over {
        return scroll;
    }
    let max = crate::list_scroll_bar::max_first(content, BATTLE_RESULT_LIST_HEIGHT);
    let row = battle_result_row_height(category);
    let moved = if wheel > 0.0 {
        scroll as f32 - row
    } else if wheel < 0.0 {
        scroll as f32 + row
    } else {
        scroll as f32
    };
    moved.clamp(0.0, max) as usize
}

/// The forces list's scroll bar for one frame: the pointer's press, hold
/// and release on it ([`crate::list_scroll_bar::update`], the list's step
/// `+0xc0` 10, `FUN_0046f360`), then the wheel.
fn battle_result_list_input(state: &mut TacticalState, content: f32, mouse: (f32, f32)) {
    let buttons = (
        is_mouse_button_pressed(MouseButton::Left),
        is_mouse_button_down(MouseButton::Left),
        is_mouse_button_released(MouseButton::Left),
    );
    battle_result_list_step(state, content, mouse, buttons, mouse_wheel().1, get_time());
}

/// [`battle_result_list_input`] for the pointer at `mouse` (logical),
/// the left button's `(pressed, down, released)`, the wheel and the time.
fn battle_result_list_step(
    state: &mut TacticalState,
    content: f32,
    mouse: (f32, f32),
    (pressed, down, released): (bool, bool, bool),
    wheel: f32,
    now: f64,
) {
    let pointer = crate::list_scroll_bar::Pointer {
        at: battle_window_local(mouse),
        pressed,
        down,
        released,
    };
    let bar = battle_result_bar(content, state.battle_result_scroll);
    let first = crate::list_scroll_bar::update(
        &bar,
        &mut state.battle_result_scroll_hold,
        pointer,
        BATTLE_RESULT_LIST_STEP,
        now,
    );
    state.battle_result_scroll = result_list_scroll(
        first as usize,
        mouse,
        wheel,
        content,
        state.battle_result_category,
    );
}

/// A logical point in the Battle Results window's coordinates.
fn battle_window_local(point: (f32, f32)) -> (f32, f32) {
    (
        point.0 - battle_window_origin().0,
        point.1 - battle_window_origin().1,
    )
}

/// The forces list's arrow step, `FUN_00607ea0`'s stack argument `0x3c`
/// (10, `FUN_0046f360`).
const BATTLE_RESULT_LIST_STEP: f32 = 10.0;

/// The arrow of the forces list's bar drawn pressed: the held arrow while
/// the pointer stays on it.
fn battle_result_pressed_arrow(
    state: &TacticalState,
    content: f32,
    mouse: (f32, f32),
    down: bool,
) -> Option<crate::list_scroll_bar::Part> {
    let held = state.battle_result_scroll_hold?.part;
    let over =
        battle_result_bar(content, state.battle_result_scroll).part_at(battle_window_local(mouse));
    (matches!(held, crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down)
        && over == Some(held)
        && down)
        .then_some(held)
}

/// Draw a results page's steps in the Battle Results window.
/// The Battle Summary's frame (10335 / 10336, keyed on blue) after the
/// page's art and before its texts: the frame's corner arcs inside the
/// aperture show over the scene in A0 (TAC-01-C006 and TAC-01-C013: 900
/// of 900 such pixels the frame's colour; empire-game3/
/// battle-result-coruscant: all 474 in the top corners).
fn frame_over_page(mut ops: Vec<PageOp>, frame: u32) -> Vec<PageOp> {
    let after_art = ops
        .iter()
        .position(|op| matches!(op, PageOp::Bitmap { .. }))
        .map_or(0, |index| index + 1);
    ops.insert(
        after_art,
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: frame,
            x: 0.0,
            y: 0.0,
        },
    );
    ops
}

fn draw_page_ops(cache: &mut BmpCache, canvas: TacticalCanvas, ops: Vec<PageOp>) {
    draw_page_ops_at(
        cache,
        canvas,
        ops,
        (battle_window_origin().0, battle_window_origin().1),
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
                bold,
            } => {
                let (x, y) = (origin_x + x, origin_y + baseline);
                let x = if centered {
                    x - battle_window_text_width_in(canvas, &text, size, bold) * 0.5 / canvas.scale
                } else {
                    x
                };
                draw_result_text_in(canvas, &text, x, y, (size, bold), color);
            }
            PageOp::Wrapped {
                lines,
                x,
                baseline,
                width,
                size,
                color,
                bold,
            } => draw_result_wrapped_lines(
                canvas,
                &lines,
                origin_x + x,
                origin_y + baseline,
                width,
                (size, bold),
                color,
            ),
            PageOp::Clip(rect) => set_tactical_aperture_clip(
                rect.map(|rect| page_clip_on_screen(canvas, (origin_x, origin_y), rect)),
            ),
        }
    }
    set_tactical_aperture_clip(None);
}

/// A page's clip rectangle (window coordinates under `origin`) on the
/// screen.
fn page_clip_on_screen(
    canvas: TacticalCanvas,
    (origin_x, origin_y): (f32, f32),
    (x, y, width, height): (f32, f32, f32, f32),
) -> NativeRect {
    let (left, top) = canvas.point(origin_x + x, origin_y + y);
    NativeRect::new(left, top, width * canvas.scale, height * canvas.scale)
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
    /// Text at its baseline; `centered` on `x`, else from it; `bold`
    /// takes the 700-weight face.
    Text {
        text: String,
        x: f32,
        baseline: f32,
        size: f32,
        color: Color,
        centered: bool,
        bold: bool,
    },
    /// Lines word-wrapped at `width` from `x`, the first at `baseline`;
    /// `bold` takes the 700-weight face.
    Wrapped {
        lines: Vec<String>,
        x: f32,
        baseline: f32,
        width: f32,
        size: f32,
        color: Color,
        bold: bool,
    },
    /// Clip the steps that follow to a rectangle (x, y, width, height),
    /// or stop clipping.
    Clip(Option<(f32, f32, f32, f32)>),
}

/// The Battle Summary's forces page as drawing steps (see
/// `draw_battle_result_force_details` for the sources); `size_of` gives a
/// bitmap's size for centring the pictures. The list starts at row
/// `scroll`.
#[expect(
    clippy::too_many_arguments,
    reason = "The page reads the battle's units, the tab and the viewer's art."
)]
fn battle_force_page(
    system_name: &str,
    entries: &[BattleResultEntry],
    alliance: bool,
    category: BattleResultCategory,
    scroll: usize,
    player_alliance: bool,
    text_color: Color,
    mut size_of: impl FnMut(DllSource, u32) -> Option<[usize; 2]>,
) -> Vec<PageOp> {
    use resources::strategy as art;
    // `cap_top` is the first capital row; the baseline sits a cap height
    // (0.7 of the em, as measured for fonts 4, 6 and 0xb) below it.
    let text = |text: &str, x: f32, cap_top: f32, size: f32, color: Color, centered: bool| {
        PageOp::Text {
            text: text.to_owned(),
            x,
            baseline: cap_top + (size * 0.7).round(),
            size,
            color,
            centered,
            bold: false,
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
        // A0 TAC-07-C005 (window coordinates): the title in font 0xb from
        // row 22; the rail label (FUN_00471200: (12, 40), 400 wide, font 6)
        // from row 45; the category label (FUN_00471200: (37, 99), font 4)
        // from row 102; the column heads (font 4) from row 121; an empty
        // column's label (font 6) from row 149; a unit's name from 7 rows
        // into its name strip. port: fonts 4 and the names are fitted to
        // the capture (em 13.75 and 11).
        text(
            &format!("Battle at {system_name}"),
            212.0,
            22.0,
            BATTLE_WINDOW_FONT_B,
            text_color,
            true,
        ),
    ];
    // FUN_00471200: rail 0x70 (Alliance) red, 0x71 (Empire) green.
    let (label, side_color) = if alliance {
        ("Alliance Forces", Color::from_rgba(255, 0, 0, 255))
    } else {
        ("Imperial Forces", Color::from_rgba(0, 255, 0, 255))
    };
    ops.push(text(
        label,
        212.0,
        45.0,
        BATTLE_WINDOW_FONT_6,
        side_color,
        true,
    ));
    ops.push(text(
        "Filters",
        86.0,
        73.0,
        BATTLE_WINDOW_FONT_4,
        WHITE,
        false,
    ));
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
    ops.push(text(
        category.title(),
        37.0,
        102.0,
        BATTLE_WINDOW_FONT_4,
        WHITE,
        false,
    ));
    let (heads, width): (&[(f32, &str)], f32) = if personnel {
        (
            &[(37.0, "Survivors"), (149.0, "Captured"), (260.0, "Killed")],
            110.0,
        )
    } else {
        (&[(37.0, "Operational"), (204.0, "Destroyed")], 165.0)
    };
    for &(x, head) in heads {
        ops.push(text(
            head,
            x + width / 2.0,
            121.0,
            BATTLE_WINDOW_FONT_4,
            WHITE,
            true,
        ));
    }

    let columns = battle_result_columns(entries, personnel);
    let (cell_w, cell_h) = if personnel { (110.0, 50.0) } else { (165.0, 70.0) };
    let empty = battle_result_empty_labels(&columns);
    ops.push(PageOp::Clip(Some((37.0, 136.0, 347.0, BATTLE_RESULT_LIST_HEIGHT))));
    for (column, units) in columns.iter().enumerate() {
        let left = 37.0 + column as f32 * cell_w;
        if let Some(label) = empty[column] {
            ops.push(text(
                label,
                left + cell_w / 2.0,
                149.0,
                BATTLE_WINDOW_FONT_6,
                WHITE,
                true,
            ));
        }
        for (row, entry) in units.iter().enumerate() {
            let top = 136.0 + row as f32 * cell_h - scroll as f32;
            // The list window (37, 136), 347 by 166, clips its cells.
            if top + cell_h <= 136.0 || top >= 136.0 + BATTLE_RESULT_LIST_HEIGHT {
                continue;
            }
            // FUN_004724a0: the picture centred in the cell, `(cell -
            // height) / 2`, and the name's label from the picture's foot
            // (`+0x34` = its height + its top). A0 TAC-07-C007: the
            // destroyed regiment's 50-high picture from window row 146 of
            // its 70-high cell at 136, its name's capitals from row 199
            // (port: the name's text box stands 3 rows under the foot).
            let mut name_y = top + cell_h - BATTLE_RESULT_NAME_HEIGHT + 7.0;
            if let Some(picture) = entry.picture {
                let mut foot = None;
                for (source, id) in entry
                    .backdrop
                    .into_iter()
                    .chain(entry.mark)
                    .chain(std::iter::once(picture))
                {
                    if let Some([w, h]) = size_of(source, id) {
                        let y = top + ((cell_h - h as f32) / 2.0).floor();
                        foot = foot.or(Some(y + h as f32));
                        ops.push(PageOp::Bitmap {
                            source,
                            id,
                            x: left + ((cell_w - w as f32) / 2.0).floor(),
                            y,
                        });
                    }
                }
                if let Some(foot) = foot {
                    name_y = foot + 3.0;
                }
            }
            ops.push(text(
                &entry.name,
                left + cell_w / 2.0,
                name_y,
                BATTLE_RESULT_NAME_FONT,
                WHITE,
                true,
            ));
        }
    }
    ops.push(PageOp::Clip(None));
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
    world: &GameWorld,
    player_alliance: bool,
    mouse: (f32, f32),
    mouse_down: bool,
    mouse_released: bool,
    text_color: Color,
) -> TacticalAction {
    let (lines, fleet) =
        battle_display_lines(&loaded_battle_texts(), &battle_report(session, world, None));
    let hovered = display_button_at(
        mouse.0 - battle_window_origin().0,
        mouse.1 - battle_window_origin().1,
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
    let frame = battle_result_skin(player_alliance).frame;
    draw_page_ops(cache, canvas, frame_over_page(ops, frame));

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

fn draw_battle_alert_text(
    state: &TacticalState,
    cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    canvas: TacticalCanvas,
    text_color: Color,
) -> usize {
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
    let origin = (battle_window_origin().0, battle_window_origin().1);
    let Some(list) = BattleAlertList::of(state.battle_alert_tab) else {
        draw_page_ops_at(
            cache,
            canvas,
            battle_alert_scene_text(&title, Some(&summary), text_color),
            origin,
        );
        return 0;
    };
    // FUN_00450660: the list tabs show the scene with the title only
    // (surface 0x33).
    draw_page_ops_at(
        cache,
        canvas,
        battle_alert_scene_text(&title, None, text_color),
        origin,
    );
    let rows = battle_alert_rows(world, troop_transport, session.system, list);
    let content = alert_list_content(rows.len());
    let (screen_x, screen_y) = mouse_position();
    let pressed = alert_pressed_arrow(
        state,
        content,
        canvas.logical_pointer(screen_x, screen_y),
        is_mouse_button_down(MouseButton::Left),
    );
    let mut ops = battle_alert_list(&rows, state.battle_alert_scroll);
    ops.extend(alert_bar_ops(&alert_list_bar(content, state.battle_alert_scroll), pressed));
    draw_page_ops_at(cache, canvas, ops, origin);
    rows.len()
}

/// The alert list's content in pixels: its 40-high rows.
/// The battle's unfinished Death Star: the current product of `system`'s
/// ship manager when it is a Death Star still being built and a fleet
/// stands at the system (`FUN_0040b0e0`, 0x40b4c9..0x40b551).
fn unfinished_death_star(
    world: &GameWorld,
    system: SystemKey,
) -> Option<&rebellion_core::products::Product> {
    if world.systems.get(system)?.fleets.is_empty() {
        return None;
    }
    let product = rebellion_core::products::current_ship_product(world, system)?;
    let rebellion_core::manufacturing::BuildableKind::CapitalShip(class) = product.kind else {
        return None;
    };
    world
        .capital_ship_classes
        .get(class)
        .is_some_and(rebellion_core::world::CapitalShipClass::is_death_star)
        .then_some(product)
}

fn alert_list_content(rows: usize) -> f32 {
    rows as f32 * ALERT_LIST_ROW
}

/// The alert list's scroll bar (`FUN_0044f860` at `0x44fd73`:
/// `FUN_0060a490(.., list, 7, 0x29fc, ..)`), in window coordinates.
fn alert_list_bar(content: f32, scroll: usize) -> crate::list_scroll_bar::ScrollBar {
    crate::list_scroll_bar::ScrollBar::new(
        crate::list_scroll_bar::ALERT_ART,
        ALERT_LIST_LEFT,
        ALERT_LIST_TOP,
        ALERT_LIST_WIDTH,
        ALERT_LIST_HEIGHT,
        content,
        scroll as f32,
    )
}

/// The alert bar's bitmaps (STRATEGY.DLL), clipped to the bar.
fn alert_bar_ops(
    bar: &crate::list_scroll_bar::ScrollBar,
    pressed: Option<crate::list_scroll_bar::Part>,
) -> Vec<PageOp> {
    battle_result_bar_ops(bar, pressed)
}

/// The alert bar's arrow drawn pressed: the held arrow while the pointer
/// stays on it.
fn alert_pressed_arrow(
    state: &TacticalState,
    content: f32,
    mouse: (f32, f32),
    down: bool,
) -> Option<crate::list_scroll_bar::Part> {
    let held = state.battle_alert_scroll_hold?.part;
    let over = alert_list_bar(content, state.battle_alert_scroll).part_at(battle_window_local(mouse));
    (matches!(held, crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down)
        && over == Some(held)
        && down)
        .then_some(held)
}

/// The alert list's bar and wheel for one frame: the list's step `+0xc0`
/// is 10 (`FUN_00607ea0`'s argument at `0x44fcd6`).
fn alert_list_step(
    (scroll, hold): (&mut usize, &mut Option<crate::list_scroll_bar::Hold>),
    content: f32,
    mouse: (f32, f32),
    (pressed, down, released): (bool, bool, bool),
    wheel: f32,
    now: f64,
) {
    let pointer = crate::list_scroll_bar::Pointer {
        at: battle_window_local(mouse),
        pressed,
        down,
        released,
    };
    let bar = alert_list_bar(content, *scroll);
    let first = crate::list_scroll_bar::update(&bar, hold, pointer, BATTLE_RESULT_LIST_STEP, now);
    *scroll = alert_list_scroll(first as usize, mouse, wheel, content);
}

/// The text `FUN_0044f860` paints into the alert's scene, which sits at
/// (12, 13): the title at (0, 16), 400 wide, centred, font `0xb`; the
/// summary at (36, 216), 328 wide, word-wrapped, font 6, over a black
/// shadow at (37, 217). Both take the viewer's colour. In window
/// coordinates the title's top is 29 and the summary's (48, 229).
/// port: font 6 keeps the port's earlier 14-point size here.
fn battle_alert_scene_text(title: &str, summary: Option<&str>, color: Color) -> Vec<PageOp> {
    // FUN_0044f860 places the title at window (0, 16), 400 wide, centred,
    // font 0xb, and the summary at window (36, 216), 328 wide, font 6, over
    // a black shadow at (37, 217): the rects are the window's, not the
    // scene's (A0 TAC-01-C010: capitals from rows 20 and 219, the
    // summary's left edge at x 36).
    let mut ops = vec![PageOp::Text {
        text: title.to_owned(),
        x: 200.0,
        baseline: 20.0 + 15.0,
        size: BATTLE_WINDOW_FONT_B,
        color,
        centered: true,
        bold: false,
    }];
    if let Some(summary) = summary {
        for (x, cap_top, color) in [(37.0, 220.0, BLACK), (36.0, 219.0, color)] {
            ops.push(PageOp::Wrapped {
                lines: vec![summary.to_owned()],
                x,
                baseline: cap_top + 12.0,
                width: 328.0,
                size: BATTLE_WINDOW_FONT_6,
                color,
                bold: false,
            });
        }
    }
    ops
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
/// A squadron rides on its ship (`FighterEntry::carrier`, save v106), so
/// it is listed under that ship. port: a squadron on no ship of the fleet,
/// and the fleet's regiments and personnel, which the port keeps on the
/// fleet, follow the fleet's ships. Facilities go by kind (defenses,
/// manufacturing, production) in the system's order.
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
                crate::minis::fighter_mini_id(class.dat_id),
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
            rebellion_core::pictures::character_picture(character),
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
                        crate::minis::capital_ship_mini_id(class.dat_id),
                        crate::status_rows::damaged(ship, class),
                        false,
                    ));
                    // FUN_005039d0: the squadrons aboard this ship.
                    rows.extend(
                        fleet
                            .fighters
                            .iter()
                            .filter(|entry| ship.id != 0 && entry.carrier == Some(ship.id))
                            .filter_map(|entry| squadron(entry, true))
                            .flatten(),
                    );
                }
                // port: squadrons on no ship of the fleet follow its ships.
                rows.extend(
                    fleet
                        .fighters
                        .iter()
                        .filter(|entry| {
                            !entry.carrier.is_some_and(|carrier| {
                                carrier != 0 && fleet.capital_ships.iter().any(|ship| ship.id == carrier)
                            })
                        })
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
            // The unfinished Death Star the system's ship manager builds
            // stands in the battle (FUN_0040b0e0 0x40b4d2 -> FUN_0040bcd0,
            // BattleSession::new_for_fleets), its mini over the unfinished
            // mark (FUN_0042c3b0: 0x2d32 / 0x2d34). port: after the side's
            // fleets.
            if let Some(product) = unfinished_death_star(world, system)
                .filter(|product| product.is_alliance == alliance)
            {
                let mini = crate::minis::capital_ship_mini_id(rebellion_core::ids::DatId::new(
                    rebellion_core::world::DEATH_STAR_CLASS_ID,
                ));
                let mut entry = row(
                    product.name.clone().unwrap_or_else(|| "Death Star".to_owned()),
                    AlertFont::Bold,
                    MiniObject::Craft,
                    mini,
                    false,
                    false,
                );
                if let Some(last) = entry.picture.pop() {
                    entry
                        .picture
                        .push(crate::fleet_window::unfinished_mini_mark(product.is_alliance));
                    entry.picture.push(last);
                }
                rows.push(entry);
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

/// The Battle Alert list's rectangle in the window (`FUN_00607ea0(..,
/// 0x53, 0x28, 0x100, 0xea)`) and the rows whose middle fits it.
const ALERT_LIST_LEFT: f32 = 83.0;
const ALERT_LIST_TOP: f32 = 40.0;
const ALERT_LIST_WIDTH: f32 = 256.0;
const ALERT_LIST_HEIGHT: f32 = 234.0;
/// `FUN_00607ea0`'s cells at `0x44fcde`: 240 by 40.
const ALERT_LIST_ROW: f32 = 40.0;
/// Arial's ascent over its cell (`usWinAscent` 1854 of 2288): GDI draws a
/// line from its cell's top, so its baseline sits that far down.
const ARIAL_ASCENT_OF_CELL: f32 = 1854.0 / 2288.0;

/// Show a tab or category; another one's list starts at its top.
fn select_resetting_scroll<T: PartialEq>(shown: &mut T, scroll: &mut usize, choice: T) {
    if *shown != choice {
        *scroll = 0;
    }
    *shown = choice;
}

/// The list's first pixel after the wheel turns `wheel` with the pointer
/// (in logical coordinates) at `pointer`: only over the list. port: the
/// original has no wheel; a notch moves a row, within the bar's range.
fn alert_list_scroll(scroll: usize, pointer: (f32, f32), wheel: f32, content: f32) -> usize {
    let (x, y) = battle_window_local(pointer);
    let over = (ALERT_LIST_LEFT..ALERT_LIST_LEFT + ALERT_LIST_WIDTH).contains(&x)
        && (ALERT_LIST_TOP..ALERT_LIST_TOP + ALERT_LIST_HEIGHT).contains(&y);
    if !over {
        return scroll;
    }
    let max = crate::list_scroll_bar::max_first(content, ALERT_LIST_HEIGHT);
    let moved = if wheel > 0.0 {
        scroll as f32 - ALERT_LIST_ROW
    } else if wheel < 0.0 {
        scroll as f32 + ALERT_LIST_ROW
    } else {
        scroll as f32
    };
    moved.clamp(0.0, max) as usize
}

/// The Battle Alert's list (`FUN_0044f860`, `FUN_00607ea0` at `0x44fd09`):
/// at (83, 40), 256 by 234, rows 40 high, white text, scrolled `scroll`
/// pixels; the list window clips its rows.
fn battle_alert_list(rows: &[AlertRow], scroll: usize) -> Vec<PageOp> {
    const LEFT: f32 = ALERT_LIST_LEFT;
    const TOP: f32 = ALERT_LIST_TOP;
    const WIDTH: f32 = ALERT_LIST_WIDTH;
    const HEIGHT: f32 = ALERT_LIST_HEIGHT;
    const ROW: f32 = ALERT_LIST_ROW;
    let mut ops = vec![PageOp::Clip(Some((LEFT, TOP, WIDTH, HEIGHT)))];
    for (index, row) in rows.iter().enumerate() {
        let top = TOP + ROW * index as f32 - scroll as f32;
        if top + ROW <= TOP || top >= TOP + HEIGHT {
            continue;
        }
        // Fonts 0xb (24 px cell), 5 (16 px, bold) and 4 (16 px)
        // (FUN_0060eed0), drawn from the cell's top (A0
        // alert-imperial-forces: "Imperial Forces" spans rows 96..113 of
        // the list's 92..132).
        let (cell, size, bold) = match row.font {
            AlertFont::Heading => (24.0, BATTLE_WINDOW_FONT_B, false),
            AlertFont::Bold => (16.0, BATTLE_WINDOW_FONT_4, true),
            AlertFont::Plain => (16.0, BATTLE_WINDOW_FONT_4, false),
        };
        let baseline = top + (cell * ARIAL_ASCENT_OF_CELL).round();
        if row.picture.is_empty() {
            ops.push(PageOp::Text {
                text: row.text.clone(),
                x: LEFT + WIDTH / 2.0,
                baseline,
                size,
                color: WHITE,
                centered: true,
                bold,
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
            bold,
        });
    }
    ops.push(PageOp::Clip(None));
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
    set_battle_window_origin(player_alliance);
    let skin = battle_alert_skin(player_alliance);
    let text_color = battle_window_text_color(player_alliance);

    draw_battle_window_backdrop(cache, canvas, player_alliance, state.strategy_underlay.as_ref());
    for (id, x, y, clip) in battle_alert_layers(&skin, state.battle_alert_tab) {
        draw_original_bitmap(
            cache,
            DllSource::Strategy,
            id,
            canvas,
            battle_window_origin().0 + x,
            battle_window_origin().1 + y,
            clip,
        );
    }
    let rows = draw_battle_alert_text(state, cache, world, troop_transport, canvas, text_color);
    let (screen_x, screen_y) = mouse_position();
    alert_list_step(
        (&mut state.battle_alert_scroll, &mut state.battle_alert_scroll_hold),
        alert_list_content(rows),
        canvas.logical_pointer(screen_x, screen_y),
        (
            is_mouse_button_pressed(MouseButton::Left),
            is_mouse_button_down(MouseButton::Left),
            is_mouse_button_released(MouseButton::Left),
        ),
        mouse_wheel().1,
        get_time(),
    );
    // A0 TAC-01-C010: the alert's tab buttons sit on frame 10710's own
    // rail; the separate rail 10820 (with its fifth socket) belongs to the
    // Battle Summary window only.

    let (screen_mouse_x, screen_mouse_y) = mouse_position();
    let mouse = canvas.logical_pointer(screen_mouse_x, screen_mouse_y);
    let local_mouse = (
        mouse.0 - battle_window_origin().0,
        mouse.1 - battle_window_origin().1,
    );
    let mouse_down = is_mouse_button_down(MouseButton::Left);
    let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);
    let mouse_released = is_mouse_button_released(MouseButton::Left);

    for (index, tab) in BattleAlertTab::ALL.into_iter().enumerate() {
        let rect = skin.tab_rects[index];
        // FUN_0044f860: Battle Summary 0x1922, Alliance Forces 0x1923,
        // Imperial Forces 0x1924, System Assets 0x1925 (TEXTSTRA).
        tactical_tool(
            canvas,
            NativeRect::new(
                battle_window_origin().0 + rect.x,
                battle_window_origin().1 + rect.y,
                rect.width,
                rect.height,
            ),
            0x0001_0000 | index as u64,
            crate::win_tooltip::StringModule::Strategy,
            0x1922 + index as u16,
            true,
        );
        let active = state.battle_alert_tab == tab
            || (mouse_down && rect.contains(local_mouse.0, local_mouse.1));
        draw_strategy_bitmap(
            cache,
            skin.tabs[index][usize::from(active)],
            canvas,
            battle_window_origin().0 + rect.x,
            battle_window_origin().1 + rect.y,
        );
        if mouse_released && rect.contains(local_mouse.0, local_mouse.1) {
            select_resetting_scroll(
                &mut state.battle_alert_tab,
                &mut state.battle_alert_scroll,
                tab,
            );
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
    let retreat_enabled = rebellion_core::abstract_battle::retreat_offered(
        world,
        session.system,
        player_is_alliance(session),
    );
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
        // FUN_0044f860: Retreat 0x194a, Simulate Results 0x1926, Take
        // Command 0x1927; a disabled Retreat shows none.
        tactical_tool(
            canvas,
            NativeRect::new(
                battle_window_origin().0 + rect.x,
                battle_window_origin().1 + rect.y,
                rect.width,
                rect.height,
            ),
            0x0001_0010 | index as u64,
            crate::win_tooltip::StringModule::Strategy,
            [0x194a, 0x1926, 0x1927][index],
            enabled,
        );
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
            battle_window_origin().0 + rect.x,
            battle_window_origin().1 + rect.y,
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
                    // The tactical view opens with its side's arrival
                    // (`FUN_005aacb0` state 2, `FUN_005a6d60`).
                    if let Some(session) = state.session.as_mut() {
                        session.queue_arrival_sounds();
                    }
                    TacticalAction::BeginCombat
                }
            };
        }
    } else if !mouse_down {
        state.battle_alert_pressed = None;
    }
    TacticalAction::None
}

/// What the Battle Alert and Battle Results windows sit on: the strategy
/// view as it last stood (`underlay`, copied at the screen's own size), or,
/// with none (a battle a fixture opens directly), the command center's
/// shell bitmap.
fn draw_battle_window_backdrop(
    cache: &mut BmpCache,
    canvas: TacticalCanvas,
    player_alliance: bool,
    underlay: Option<&Texture2D>,
) {
    clear_background(BLACK);
    if let Some(underlay) = underlay.filter(|texture| {
        (texture.width() - screen_width()).abs() < 0.5 && (texture.height() - screen_height()).abs() < 0.5
    }) {
        // A screen copy keeps the framebuffer's bottom-up rows.
        draw_texture_ex(
            underlay,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );
        return;
    }
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
}

fn draw_original_battle_results(
    state: &mut TacticalState,
    cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
    backdrop: bool,
) -> TacticalAction {
    let canvas = TacticalCanvas::new(screen_width(), screen_height());
    let session = state.session.as_ref().expect("active result session");
    prewarm_tactical_font_atlas(session, canvas, &mut state.warmed_font_sizes);
    let player_alliance = player_is_alliance(session);
    set_battle_window_origin(player_alliance);
    let skin = battle_result_skin(player_alliance);
    let text_color = battle_window_text_color(player_alliance);

    // The frame is painted with each page, over its art (`frame_over_page`).
    if backdrop {
        if !state.summary_over_tactical_logged {
            state.summary_over_tactical_logged = true;
            macroquad::logging::info!("[tactical-results] summary_window over=tactical_view");
        }
        draw_battle_window_backdrop(cache, canvas, player_alliance, state.strategy_underlay.as_ref());
    }

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
            let entries = battle_result_entries(
                session,
                world,
                troop_transport,
                true,
                state.battle_result_category,
            );
            let content = battle_result_content(&entries, state.battle_result_category);
            let pressed = battle_result_pressed_arrow(state, content, mouse, mouse_down);
            draw_battle_result_force_details(
                cache,
                canvas,
                session,
                world,
                troop_transport,
                true,
                state.battle_result_category,
                state.battle_result_scroll,
                pressed,
                player_alliance,
                text_color,
            );
            battle_result_list_input(state, content, mouse);
            TacticalAction::None
        }
        BattleResultTab::ImperialForces => {
            let entries = battle_result_entries(
                session,
                world,
                troop_transport,
                false,
                state.battle_result_category,
            );
            let content = battle_result_content(&entries, state.battle_result_category);
            let pressed = battle_result_pressed_arrow(state, content, mouse, mouse_down);
            draw_battle_result_force_details(
                cache,
                canvas,
                session,
                world,
                troop_transport,
                false,
                state.battle_result_category,
                state.battle_result_scroll,
                pressed,
                player_alliance,
                text_color,
            );
            battle_result_list_input(state, content, mouse);
            TacticalAction::None
        }
        BattleResultTab::GoDirectlyTo => draw_battle_result_destinations(
            cache,
            canvas,
            session,
            world,
            player_alliance,
            mouse,
            mouse_down,
            mouse_released,
            text_color,
        ),
    };

    // The window's last row, all blue in its frame, shows the window's own
    // black.
    let row = BATTLE_WINDOW_UNPAINTED_ROW;
    let (row_x, row_y) = canvas.point(
        battle_window_origin().0 + row.x,
        battle_window_origin().1 + row.y,
    );
    draw_rectangle(
        row_x,
        row_y,
        row.width * canvas.scale,
        row.height * canvas.scale,
        BLACK,
    );
    draw_strategy_bitmap(
        cache,
        skin.rail,
        canvas,
        battle_window_origin().0 + BATTLE_RESULTS_RAIL_X,
        battle_window_origin().1,
    );
    let close_active = mouse_down
        && skin.close_rect.contains(
            mouse.0 - battle_window_origin().0,
            mouse.1 - battle_window_origin().1,
        );
    draw_strategy_bitmap(
        cache,
        skin.close[usize::from(close_active)],
        canvas,
        battle_window_origin().0 + skin.close_rect.x,
        battle_window_origin().1 + skin.close_rect.y,
    );
    if mouse_released
        && skin.close_rect.contains(
            mouse.0 - battle_window_origin().0,
            mouse.1 - battle_window_origin().1,
        )
    {
        action = TacticalAction::ReturnToGalaxy;
    }

    for (index, rect) in skin.tab_rects.into_iter().enumerate() {
        let local_mouse = (
            mouse.0 - battle_window_origin().0,
            mouse.1 - battle_window_origin().1,
        );
        // FUN_0046f360: Battle Summary 0x1922, Alliance Forces 0x1937,
        // Imperial Forces 0x1938, Display 0x1939 (TEXTSTRA).
        tactical_tool(
            canvas,
            NativeRect::new(
                battle_window_origin().0 + rect.x,
                battle_window_origin().1 + rect.y,
                rect.width,
                rect.height,
            ),
            0x0002_0000 | index as u64,
            crate::win_tooltip::StringModule::Strategy,
            [0x1922, 0x1937, 0x1938, 0x1939][index],
            true,
        );
        let selected = state.battle_result_tab.index() == index;
        let active = selected || (mouse_down && rect.contains(local_mouse.0, local_mouse.1));
        draw_strategy_bitmap(
            cache,
            skin.tabs[index][usize::from(active)],
            canvas,
            battle_window_origin().0 + rect.x,
            battle_window_origin().1 + rect.y,
        );
        if mouse_released && rect.contains(local_mouse.0, local_mouse.1) {
            select_resetting_scroll(
                &mut state.battle_result_tab,
                &mut state.battle_result_scroll,
                BattleResultTab::from_index(index),
            );
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
    ) {
        // FUN_0046f360: Capital Ships 0x1928, Fighter Squadrons 0x1929,
        // Trooper Regiments 0x1930, Personnel 0x1931.
        for index in 0..BattleResultCategory::ALL.len() {
            let rect = battle_category_rect(index);
            tactical_tool(
                canvas,
                NativeRect::new(
                    battle_window_origin().0 + rect.x,
                    battle_window_origin().1 + rect.y,
                    rect.width,
                    rect.height,
                ),
                0x0002_0010 | index as u64,
                crate::win_tooltip::StringModule::Strategy,
                [0x1928, 0x1929, 0x1930, 0x1931][index],
                true,
            );
        }
    }
    // The window's key slot (vtable 0x65a300 +0x4c, FUN_004710f0): Escape
    // closes it as Close does; the arrows step the category bar.
    let forces_page = matches!(
        state.battle_result_tab,
        BattleResultTab::AllianceForces | BattleResultTab::ImperialForces
    );
    match battle_summary_key(
        is_key_pressed(KeyCode::Escape),
        is_key_pressed(KeyCode::Left),
        is_key_pressed(KeyCode::Right),
    ) {
        Some(BattleSummaryKey::Close) => {
            macroquad::logging::info!("[tactical-results] key=escape status=closed");
            action = TacticalAction::ReturnToGalaxy;
        }
        Some(BattleSummaryKey::Step { right }) => {
            if let Some(category) =
                battle_category_after_arrow(state.battle_result_category, right, forces_page)
            {
                select_resetting_scroll(
                    &mut state.battle_result_category,
                    &mut state.battle_result_scroll,
                    category,
                );
                macroquad::logging::info!(
                    "[tactical-results] tab={} category={} key={}",
                    state.battle_result_tab.label(),
                    category.label(),
                    if right { "right" } else { "left" }
                );
            }
        }
        None => {}
    }

    if matches!(
        state.battle_result_tab,
        BattleResultTab::AllianceForces | BattleResultTab::ImperialForces
    ) && mouse_released
    {
        for (index, category) in BattleResultCategory::ALL.into_iter().enumerate() {
            let rect = battle_category_rect(index);
            if rect.contains(
                mouse.0 - battle_window_origin().0,
                mouse.1 - battle_window_origin().1,
            ) {
                select_resetting_scroll(
                    &mut state.battle_result_category,
                    &mut state.battle_result_scroll,
                    category,
                );
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

/// Whether a tactical frame draws the Battle Summary: only when the battle
/// had already ended as the frame began (the app moves such a battle to the
/// Command Center first). The frame whose step ends the battle draws the
/// battle itself (FUN_0040a050 state 0x16 closes the module before
/// FUN_0042a640 opens the summary on the Command Center).
const fn battle_frame_draws_summary(results_at_start: bool) -> bool {
    results_at_start
}

/// The Battle Summary's row under its frame: `FUN_0046f140` makes the
/// window 331 rows tall, its frames 10335 / 10336 are 331 rows with the
/// last one all blue (keyed), and no page paints it: A0 TAC-07-C001,
/// TAC-07-C003..C009 (Alliance, at (62, 49)), TAC-07-C002 (Empire, at
/// (125, 52)) and 2026-11-18-edited-saves-7 d-14 (56b287e7cb22) show all
/// 470 of its pixels black.
const BATTLE_WINDOW_UNPAINTED_ROW: NativeRect = NativeRect::new(0.0, 330.0, 470.0, 1.0);

/// The Battle Summary window's rect on the 640 x 480 strategy screen: its
/// top left where it opens (A0 TAC-01-C013, TAC-07-C001 and TAC-07-C002),
/// and `FUN_0046f140`'s size, `0x1d6` by `0x14b` (470 by 331).
#[must_use]
pub fn battle_summary_window_rect(player_alliance: bool) -> Rect {
    let (x, y) = if player_alliance {
        BATTLE_WINDOW_ALLIANCE
    } else {
        BATTLE_WINDOW_EMPIRE
    };
    Rect::new(x, y, 470.0, 331.0)
}

/// Draw the Battle Summary as a window over the strategy screen the app has
/// already drawn, then the shared tooltip over it.
///
/// The original opens it after the battle as a child window of the Command
/// Center (`FUN_0042a640` on `DAT_006b14cc`: `FUN_0046f140`, then
/// `FUN_005ffce0`, `FUN_005f4f10` and `SetFocus`), as the System Finder's
/// `FUN_0042a000` opens its window: no backdrop of its own.
///
/// # Panics
/// Panics if no battle is in its results.
pub fn draw_battle_summary_window(
    state: &mut TacticalState,
    bmp_cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
) -> TacticalAction {
    assert!(state.battle_summary_open(), "no Battle Summary to draw");
    let action = draw_original_battle_results(state, bmp_cache, world, troop_transport, false);
    paint_tactical_tip(TacticalCanvas::new(screen_width(), screen_height()));
    action
}

/// Draw the tactical combat view (macroquad), then the shared tooltip over
/// it.
///
/// Call this when `GameMode::TacticalCombat`. Returns a `TacticalAction`
/// indicating whether the main loop should transition modes.
///
/// # Panics
/// Panics if the active battle session disappears during drawing.
pub fn draw_tactical_view(
    state: &mut TacticalState,
    bmp_cache: &mut BmpCache,
    world: &GameWorld,
    troop_transport: &TroopTransportState,
) -> TacticalAction {
    let action = draw_tactical_frame(state, bmp_cache, world, troop_transport);
    paint_tactical_tip(TacticalCanvas::new(screen_width(), screen_height()));
    action
}

/// One frame of the battle's clocks, `dt` seconds long.
///
/// The Pause Battle button (command 0xad22, `FUN_0059b130`) only counts
/// `+0x8c8` up or down (`FUN_005aac30`/`FUN_005aac60`); it does not stop
/// the scene render `FUN_005c25b0`. Each render still runs `FUN_005c2c80`,
/// which steps the field frames (`FUN_005d3cc0`), the impact sequences
/// (`FUN_005d41a0`) and the superlaser beam (`0x005da830`), none of which
/// tests `+0x8c8`, and moves the scene (`IDirect3DRMFrame::Move`, "Moving
/// scene failed"), whose move callback `LAB_005eeb90` carries each
/// projectile on its own wall-clock timer. So those presentational
/// effects run on while paused; the combat steps do not. The turn clock
/// runs on too (`BattleSession::resume_paused_turn_clock`), so a turn that
/// fell due during the pause begins on the resumed frame.
/// `freeze_paused_effects` is the still fixtures' switch only; it also
/// holds that clock.
fn advance_tactical_frame(
    session: &mut BattleSession,
    previous_poses: &mut Option<TacticalPoses>,
    dt: f32,
    freeze_paused_effects: bool,
) {
    if session.phase != BattlePhase::Combat {
        return;
    }
    // FUN_005ada10 runs on every manager update, paused or not.
    session.advance_voice_announcer(dt * 1000.0);
    if !(session.paused && freeze_paused_effects) {
        advance_presentational_effects_logging_hull_fire(session, dt);
    }
    if session.paused {
        if !freeze_paused_effects && session.turn_clock_started() {
            session.paused_turn_seconds += dt;
        }
        return;
    }
    let waiting = session.end_wait_ms.is_some();
    // Step at ~4 ticks per second (at 60fps), scaled by combat_speed.
    print_battle_diagnostics(session);
    let ticks_per_sec = 4.0 * session.combat_speed as f32;
    session.resume_paused_turn_clock(ticks_per_sec);
    session.step_accumulator += dt * ticks_per_sec;
    while session.step_accumulator >= 1.0 {
        session.step_accumulator -= 1.0;
        *previous_poses = Some(TacticalPoses::of(session));
        session.step();
        print_battle_diagnostics(session);
        if session.phase == BattlePhase::Results {
            info!("[tactical-end] battle_ended tick={}", session.combat_tick);
            break;
        }
    }
    if let Some(wait) = session.end_wait_ms.filter(|_| !waiting) {
        info!(
            "[tactical-end] wait_started ms={wait} tick={}",
            session.combat_tick
        );
    }
}

/// `advance_presentational_effects`, logging each destroyed hull's fire
/// that runs out, and whether the battle was paused then.
fn advance_presentational_effects_logging_hull_fire(session: &mut BattleSession, dt: f32) {
    let burning = |session: &BattleSession| -> Vec<usize> {
        session
            .impact_effects
            .iter()
            .filter(|effect| effect.sequence == OriginalTacticalEffectSequence::Destroyed)
            .map(|effect| effect.target)
            .collect()
    };
    let before = burning(session);
    session.advance_presentational_effects(dt);
    if before.is_empty() {
        return;
    }
    let after = burning(session);
    for target in before.into_iter().filter(|target| !after.contains(target)) {
        info!(
            "[tactical-end] hull_fire_ended target={target} paused={} tick={}",
            session.paused, session.combat_tick
        );
    }
}

/// The battle screens' frame, without the tooltip.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
fn draw_tactical_frame(
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
    // The original never shows the summary in the tactical view: state
    // 0x16 closes the module (FUN_005975a0) before FUN_0042a640 creates the
    // summary on the Command Center. A battle that ends in this frame's
    // step draws its last frame here; the app opens the summary next frame.
    let results_at_start = state
        .session
        .as_ref()
        .is_some_and(|session| session.phase == BattlePhase::Results);

    // 0. Advance combat if in Combat phase (mutable borrow).
    {
        let freeze_paused_effects = state.freezes_paused_effects();
        let session = state.session.as_mut().unwrap();
        advance_tactical_frame(
            session,
            &mut state.previous_poses,
            get_frame_time(),
            freeze_paused_effects,
        );
    }

    if state
        .session
        .as_ref()
        .is_some_and(|session| session.phase == BattlePhase::Results)
        && battle_frame_draws_summary(results_at_start)
    {
        // A caller that keeps a finished battle in the tactical view.
        return draw_original_battle_results(state, bmp_cache, world, troop_transport, true);
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
    // The collision envelopes settle the participants before the camera
    // looks at one of them.
    if state.render_original_participants {
        apply_original_collision_envelopes(state);
    }
    if !update_tactical_camera_follow(state) {
        track_tactical_camera_target(state);
    }
    if state.render_original_starfield {
        state.asset_renderer.draw_backdrop(aperture_tuple);
    }
    if state.render_original_planet {
        if let Some(session) = state.session.as_ref() {
            let extent = session.source_layout.battle_extent;
            let (center, size) =
                planet_backdrop_source(extent, state.backdrop_offsets(extent).planet);
            state
                .asset_renderer
                .draw_planet(aperture_tuple, center.rendered(), size);
        }
    }
    draw_tactical_death_star_backdrop(state, aperture_tuple);
    if state.display_holocube {
        draw_tactical_holocube(state, aperture_tuple, canvas.scale);
    }

    // 2. Submit mapped production participants through their original 3D and
    // type-303 resource families. Unmapped or unavailable objects retain the
    // bounded 2D fallback below.
    let (scale, offset_x, offset_y) =
        canvas.arena_transform(state.zoom, state.camera_x, state.camera_y);
    let render_pyrotechnics = state.render_pyrotechnics;
    let previous_poses = state
        .session
        .as_ref()
        .and_then(|session| state.previous_poses.as_ref()?.before_latest_step(session));
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
                let fraction = session.step_accumulator;
                let ship_pose = |index: usize| {
                    let ship = &session.ships[index];
                    presented_pose(
                        previous_poses.and_then(|poses| poses.ships.get(index).copied()),
                        (ship.source_position, ship.source_forward),
                        fraction,
                    )
                };
                let fighter_position = |index: usize| {
                    let current = session.fighters[index].source_position;
                    presented_pose(
                        previous_poses
                            .and_then(|poses| poses.fighters.get(index).copied())
                            .map(|position| (position, TacticalWorldVector::SOURCE_FORWARD)),
                        (current, TacticalWorldVector::SOURCE_FORWARD),
                        fraction,
                    )
                    .0
                };
                let ships = session
                    .ships
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| hull_in_scene(session, *index))
                    .filter_map(|(index, ship)| {
                        let (position, forward) = ship_pose(index);
                        ship.tactical_resource.map(|resource| TacticalRenderObject {
                            object_id: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                            resource_base: resource.mesh_resource_base,
                            position: position.rendered(),
                            axes: original_hull_axes(macroquad::math::vec3(
                                forward.x, forward.y, forward.z,
                            )),
                            highlight: tactical_highlight_color(
                                ship.is_attacker == session.attacker_is_alliance,
                                (state.highlight_alliance, state.highlight_empire),
                            ),
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
                                position: fighter_position(index).rendered(),
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let effects = if render_pyrotechnics {
                    session
                        .impact_effects
                        .iter()
                        .filter_map(|effect| {
                            session.ships.get(effect.target)?;
                            Some(TacticalDecalRenderObject {
                                object_id: u32::try_from(effect.target)
                                    .unwrap_or(u32::MAX)
                                    .saturating_add(1),
                                resource_id: effect.resource_id(),
                                decal_size: effect.sequence.decal_size(),
                                decal_origin: effect.sequence.decal_origin(),
                                camera_offset: ORIGINAL_IMPACT_DECAL_CAMERA_OFFSET,
                                position: ship_pose(effect.target).0.rendered(),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let fields = session
                    .field_effects
                    .iter()
                    .filter(|effect| shows_field_cloud(effect.target))
                    .filter_map(|effect| {
                        // The cloud sits on its ship (`FUN_005c1360`), with
                        // the ship's id; a fighter group shows none
                        // ([`shows_field_cloud`]).
                        let TacticalUnitRef::Ship(index) = effect.target else {
                            return None;
                        };
                        session.ships.get(index)?;
                        let (object_id, position) = (
                            u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                            ship_pose(index).0,
                        );
                        Some(TacticalDecalRenderObject {
                            object_id,
                            resource_id: effect.resource_id()?,
                            decal_size: ORIGINAL_FIELD_DECAL_SIZE,
                            decal_origin: ORIGINAL_FIELD_DECAL_ORIGIN,
                            camera_offset: ORIGINAL_FIELD_DECAL_CAMERA_OFFSET,
                            position: position.rendered(),
                        })
                    })
                    .collect::<Vec<_>>();
                let projectiles = session
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
    draw_tactical_death_star_beam(state, aperture_tuple);
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
            BattleSession::refresh_original_task_force_geometry(
                &mut session.ships,
                &session.task_force_formations,
            );
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
    state.asset_renderer.draw_navigation_markers(
        (aperture.x, aperture.y, aperture.width, aperture.height),
        &tactical_navigation_markers(
            session,
            &navigation_projections,
            state.selected_navigation_point,
        ),
    );

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
            match enemy_selection_frame_color(session, ship) {
                None => draw_projected_selection_frame(*projection, canvas.scale),
                Some(color) => draw_projected_frame(*projection, canvas.scale, color),
            }
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
            &mut state.selected_navigation_point,
            scale,
            offset_x,
            offset_y,
            canvas.aperture(),
            &ship_projections,
            &fighter_projections,
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
        let orders_line = groups.panel_ship_name().map(|_| groups.panel_orders_line());
        if state.logged_orders_line != orders_line {
            if let Some(line) = &orders_line {
                macroquad::logging::info!("[tactical_panel] orders_line={}", line);
            }
            state.logged_orders_line = orders_line;
        }
        let death_star = state.session.as_ref().and_then(player_death_star);
        let has_selection =
            groups.selected_fighter_group.is_some() || groups.selected_ship_name.is_some();
        let attack_death_star_enabled = state
            .session
            .as_ref()
            .is_some_and(attack_death_star_enabled);
        let gates = BattleOptionsGates::of(state.session.as_ref());
        let pressed_options = state
            .battle_options_open
            .then(|| pressed_tactical_battle_options_control(bmp_cache, canvas, gates))
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
            gates,
            pressed_options,
            pressed_withdraw_confirmation
                .flatten()
                .filter(|control| state.withdraw_confirmation_pressed == Some(*control)),
            death_star,
            state.death_star_laser_pressed,
            state.death_star_targeting,
            contents_scroll_of(state.contents_scroll, groups.selected_ship_index),
            state.asset_renderer.battle_palette(),
        );
        // A0 TAC-01-C003/C002: "Battle Paused", capitals from row 33 at
        // x 22, pure red for the Alliance and pure green for the Empire;
        // TAC-06-C005: "Observe Battle" in white, its capitals from row 33,
        // ending at x 451.
        if paused {
            draw_tactical_status_text(
                canvas,
                "Battle Paused",
                22.0,
                tactical_pause_label_color(player_is_alliance),
            );
        }
        if state.player_observing {
            let width = tactical_status_text_width(canvas, "Observe Battle");
            draw_tactical_status_text(canvas, "Observe Battle", 451.0 - width, WHITE);
        }
    }

    // The battle ended in this frame's step: its controls take no input.
    if state
        .session
        .as_ref()
        .is_some_and(|session| session.phase == BattlePhase::Results)
    {
        return TacticalAction::None;
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

/// The holocube's twelve edges. `FUN_005d4d10` builds two blue (0, 0, 1)
/// wireframe mesh builders (quality `0x40`) from the unit cube's eight
/// +-0.5 vertices, one facing out and one in, and scales both by the
/// battle extent `+0x8cc` that `FUN_005ab650` and `0x5ac5bc` pass through
/// `FUN_005c1d30`; `FUN_005d58c0` adds them when option `0x2000000` is on.
/// The cube therefore spans +-extent/2 on every axis.
fn tactical_holocube_edges(extent: f32) -> [(Vec3, Vec3); 12] {
    let h = extent * 0.5;
    let corner = |i: u8| {
        vec3(
            if i & 1 == 0 { -h } else { h },
            if i & 2 == 0 { -h } else { h },
            if i & 4 == 0 { -h } else { h },
        )
    };
    let mut edges = [(Vec3::ZERO, Vec3::ZERO); 12];
    let mut next = 0;
    for a in 0..8_u8 {
        for bit in [1_u8, 2, 4] {
            if a & bit == 0 {
                edges[next] = (corner(a), corner(a | bit));
                next += 1;
            }
        }
    }
    edges
}

/// port: the original's lines are retained-mode meshes depth-tested with the
/// ships; the port draws them before the ships, so every ship covers them.
/// Each edge is split so that the parts in front of the camera survive the
/// projection; A0 TAC-01-C003 shows them pure blue, one pixel wide.
const TACTICAL_HOLOCUBE_STEPS: u16 = 64;

/// Clip a screen segment to `(x0, y0, x1, y1)` (Liang-Barsky).
fn clip_segment_to_rect(a: Vec2, b: Vec2, rect: (f32, f32, f32, f32)) -> Option<(Vec2, Vec2)> {
    let (x0, y0, x1, y1) = rect;
    let delta = b - a;
    let mut enter = 0.0_f32;
    let mut leave = 1.0_f32;
    for (p, q) in [
        (-delta.x, a.x - x0),
        (delta.x, x1 - a.x),
        (-delta.y, a.y - y0),
        (delta.y, y1 - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                enter = enter.max(t);
            } else {
                leave = leave.min(t);
            }
        }
    }
    (enter <= leave).then(|| (a + delta * enter, a + delta * leave))
}

/// `FUN_005ab0e0` gives each hull its collision envelope when the tactical
/// object is created, before the first frame; the paused opening formation
/// is laid out with it.
fn apply_original_collision_envelopes(state: &mut TacticalState) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let mut changed = false;
    for ship in session.ships.iter_mut().filter(|ship| ship.alive) {
        let Some(resource) = ship.tactical_resource else {
            continue;
        };
        let Some(envelope) = state
            .asset_renderer
            .participant_collision_envelope(resource.mesh_resource_base)
        else {
            continue;
        };
        let recovered = OriginalTacticalCollisionEnvelope {
            vertical_radius: envelope.vertical_radius,
            planar_diameter: envelope.planar_diameter,
        };
        if ship.source_collision_envelope != Some(recovered) {
            ship.source_collision_envelope = Some(recovered);
            changed = true;
        }
    }
    if changed && session.combat_tick == 0 && session.paused {
        BattleSession::refresh_original_task_force_geometry(
            &mut session.ships,
            &session.task_force_formations,
        );
    }
}

/// How long `0x005da830` grows the superlaser beam: its timer is clamped
/// to `DAT_0066d4bc` (1.0 s), and the beam ends when the timer passes it.
const DEATH_STAR_BEAM_GROW_SECONDS: f32 = 1.0;

/// The drawn share of the beam's length: none before event `0x44` starts
/// it, and none once `0x005da830` has grown it for its second.
fn death_star_beam_progress(beam: &rebellion_core::tactical::TacticalDeathStarBeam) -> Option<f32> {
    (beam.fired && beam.elapsed <= DEATH_STAR_BEAM_GROW_SECONDS)
        .then(|| beam.elapsed / DEATH_STAR_BEAM_GROW_SECONDS)
}

/// The beam's part behind the Death Star decal's depth, drawn just before
/// the decal so its opaque pixels hide it (`draw_death_star_beam`).
fn draw_tactical_death_star_beam_behind(state: &mut TacticalState, aperture: (f32, f32, f32, f32)) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Some(beam) = session.death_star_beam else {
        return;
    };
    let Some(progress) = death_star_beam_progress(&beam) else {
        return;
    };
    let extent = session.source_layout.battle_extent;
    let offsets = state.backdrop_offsets(extent).death_star;
    let (decal, _) = death_star_backdrop_source(extent, offsets);
    state.asset_renderer.draw_death_star_beam(
        aperture,
        death_star_dish_source(extent, offsets).rendered(),
        decal.rendered(),
        beam.target_position.rendered(),
        progress,
        extent,
        true,
    );
}

fn draw_tactical_death_star_beam(state: &mut TacticalState, aperture: (f32, f32, f32, f32)) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Some(beam) = session.death_star_beam else {
        return;
    };
    let Some(progress) = death_star_beam_progress(&beam) else {
        return;
    };
    let extent = session.source_layout.battle_extent;
    let offsets = state.backdrop_offsets(extent).death_star;
    let (decal, _) = death_star_backdrop_source(extent, offsets);
    let drawn = state.asset_renderer.draw_death_star_beam(
        aperture,
        death_star_dish_source(extent, offsets).rendered(),
        decal.rendered(),
        beam.target_position.rendered(),
        progress,
        extent,
        false,
    );
    if !state.logged_death_star_beam {
        macroquad::logging::info!(
            "[tactical_3d] death_star_beam target_object_id={} grow_seconds={} {} source=FUN_005da150,0x005da830",
            beam.target + 1,
            DEATH_STAR_BEAM_GROW_SECONDS,
            drawn.map_or_else(
                || "visible=false".to_owned(),
                |(start, tip)| format!(
                    "visible=true start={:.1},{:.1} tip={:.1},{:.1}",
                    start.x, start.y, tip.x, tip.y
                ),
            ),
        );
        state.logged_death_star_beam = true;
    }
}

/// Whether a field (tractor or gravity) on `target` shows its cloud.
/// `FUN_005c1360` asks the unit's scene object for vtable `0x24`. Only the
/// capital ship class `0x66d198` has `FUN_005d3ac0` there (its one pointer,
/// `0x66d1bc`); the fighter group class `0x66d1f8` (`FUN_005d43c0`, built
/// by `FUN_005c1fc0`) has `0x403ef0`, a bare `ret 8`. The field still
/// holds the group; only the cloud is missing.
const fn shows_field_cloud(target: TacticalUnitRef) -> bool {
    matches!(target, TacticalUnitRef::Ship(_))
}

/// Where `FUN_005d4d10` hangs the planet decal, in source coordinates, and
/// its width, for a battle extent `e`: a frame at
/// `(2e, rand(0.2e) - 0.1e, rand(0.2e) - 0.1e)` (`DAT_0066d23c` 0.1)
/// carrying an `e` decal, built only while Show Planet is on (`+0x680`).
/// `offset` is the frame's (y, z) offset, drawn from the clock-seeded
/// stream by [`BackdropOffsets::from_clock`].
fn planet_backdrop_source(extent: f32, offset: (f32, f32)) -> (TacticalWorldPosition, f32) {
    (
        TacticalWorldPosition {
            x: 2.0 * extent,
            y: offset.0,
            z: offset.1,
        },
        extent,
    )
}

/// Where `FUN_005d4d10` hangs the Death Star decal, in source coordinates,
/// and its width, for a battle extent `e`: the frame `+0x50` at
/// `(-2e, rand(0.4e) - 0.2e, rand(0.4e) - 0.2e)` (`DAT_0066d248` -2,
/// `DAT_0066d244` 0.2), opposite the planet, which carries the `0.5e`
/// decal (`Class_0066d278_vt8`, `0x005d5970`: AddVisual of the texture
/// `+0x64` on `+0x50`). `offset` is the frame's (y, z) offset, drawn from
/// the clock-seeded stream by [`BackdropOffsets::from_clock`].
fn death_star_backdrop_source(extent: f32, offset: (f32, f32)) -> (TacticalWorldPosition, f32) {
    (
        TacticalWorldPosition {
            x: -2.0 * extent,
            y: offset.0,
            z: offset.1,
        },
        0.5 * extent,
    )
}

/// The dish the superlaser leaves from: `FUN_005d4d10`'s child frame `+0x4c`
/// at `(0, 0.11e, -0.09e)` (`DAT_0066d258`, `DAT_0066d250`) on the decal's
/// frame, the frame `FUN_005d59e0` hands `FUN_005da150` for the beam. A0
/// 2026-11-18-edited-saves-8 l-07 (377f230464ae) shows the beam's star and
/// line leaving above the sphere's centre.
fn death_star_dish_source(extent: f32, offset: (f32, f32)) -> TacticalWorldPosition {
    let (decal, _) = death_star_backdrop_source(extent, offset);
    TacticalWorldPosition {
        y: decal.y + 0.11 * extent,
        z: decal.z - 0.09 * extent,
        ..decal
    }
}

fn draw_tactical_death_star_backdrop(state: &mut TacticalState, aperture: (f32, f32, f32, f32)) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Some(death_star) = session.death_star else {
        return;
    };
    let extent = session.source_layout.battle_extent;
    let (center, size) =
        death_star_backdrop_source(extent, state.backdrop_offsets(extent).death_star);
    let resource_id = death_star.resource_id();
    draw_tactical_death_star_beam_behind(state, aperture);
    let drawn = state
        .asset_renderer
        .draw_decal(aperture, resource_id, center.rendered(), size);
    let cell = drawn.map(|(screen, _)| ((screen.x / 16.0).floor() as i32, (screen.y / 16.0).floor() as i32));
    if state.logged_death_star_backdrop != Some((resource_id, cell)) {
        let placement = drawn.map_or_else(
            || "visible=false".to_owned(),
            |(screen, size)| {
                format!(
                    "visible=true center={:.1},{:.1} size={:.1}",
                    screen.x, screen.y, size.x
                )
            },
        );
        macroquad::logging::info!(
            "[tactical_3d] death_star_backdrop resource={} {} source=FUN_005ab0e0,FUN_005c1d30,FUN_005d4d10",
            resource_id,
            placement,
        );
        state.logged_death_star_backdrop = Some((resource_id, cell));
    }
}

fn draw_tactical_holocube(state: &TacticalState, aperture: (f32, f32, f32, f32), scale: f32) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let color = Color::new(0.0, 0.0, 1.0, 1.0);
    // Keep the whole line width inside the retained-mode viewport, the
    // black rect the original's lines stay in (A0 TAC-01-C008).
    let inset = scale.ceil() * 0.5 + 0.5;
    let (x, y, width, height) = crate::tactical_assets::tactical_scene_viewport(aperture);
    let inner = (x + inset, y + inset, x + width - inset, y + height - inset);
    for (from, to) in tactical_holocube_edges(session.source_layout.battle_extent) {
        let mut previous: Option<Vec2> = None;
        for step in 0..=TACTICAL_HOLOCUBE_STEPS {
            let point = from.lerp(to, f32::from(step) / f32::from(TACTICAL_HOLOCUBE_STEPS));
            let screen = state
                .asset_renderer
                .project_source_position(aperture, point);
            if let (Some(a), Some(b)) = (previous, screen) {
                if let Some((a, b)) = clip_segment_to_rect(a, b, inner) {
                    draw_line(a.x, a.y, b.x, b.y, scale, color);
                }
            }
            previous = screen;
        }
    }
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

/// A click on a navigation point, posted to the selection state's slot
/// `+0x10` with Ctrl (`FUN_005c2060` posts a Ctrl left click and a right
/// click alike; `0x005a1f30`, `0x005a3f00`, `0x005a4e50`).
fn click_navigation_point(
    session: &mut BattleSession,
    navigation: &TacticalNavigationProjection,
    control: bool,
) -> (usize, usize, bool) {
    if session.selected_fighter_group.is_some() {
        let members = assign_selected_fighter_group_route(session, navigation.position, control);
        macroquad::logging::info!(
            "[tactical_navigation] event=fighter_route_assign set={} point={} append={} members={}",
            navigation.set + 1,
            navigation.point + 1,
            control,
            members,
        );
        return (members, 0, false);
    }
    let several = route_selected_ships_as_a_force(session);
    if !several {
        queue_capital_route_voice(session, navigation.position, control);
    }
    let mut assigned = 0;
    let mut longest_route = 0;
    let mut removed = false;
    for ship in &mut session.ships {
        if !ship.selected || !ship.alive || ship.is_attacker != session.player_is_attacker {
            continue;
        }
        ship.order = TacticalOrder::None;
        ship.escort_target = None;
        if control && !several {
            removed |= toggle_route_point(&mut ship.navigation_route, navigation.position);
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
        "[tactical_navigation] event={} set={} point={} append={} members={} route_len={}",
        if removed { "route_remove" } else { "route_assign" },
        navigation.set + 1,
        navigation.point + 1,
        control,
        assigned,
        longest_route,
    );
    (assigned, longest_route, removed)
}

/// Several selected ships given a point (`0x005a3020`, the several-units
/// state's slot `+0x10`, Ctrl or not) form a new task force in the first
/// free slot (`FUN_0059f610`), whose line plays (`0x22`/`0x9c` plus the
/// slot); with none free each ship takes a force of its own and the
/// `0x84`/`0x102` line plays. Either way the point is the new force's
/// whole route. Returns whether the selection was several ships.
fn route_selected_ships_as_a_force(session: &mut BattleSession) -> bool {
    if session.task_force_selection.is_some() {
        return false;
    }
    let player_is_attacker = session.player_is_attacker;
    let members = session
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.selected && ship.alive && ship.is_attacker == player_is_attacker)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if members.len() < 2 {
        return false;
    }
    form_selected_task_force(session, &members);
    let faction = session.player_voice_faction();
    let cue = match session.task_force_selection {
        Some(slot) => TacticalVoiceCue::capital_maneuver(faction, slot),
        None => TacticalVoiceCue::target_task_force_rejected(faction),
    };
    session.pending_voice_cues.push(cue);
    true
}

/// The routes of the player's selected ships and squadrons, which the
/// navigation display marks.
fn selected_routes(session: &BattleSession) -> Vec<&[TacticalWorldPosition]> {
    session
        .ships
        .iter()
        .filter(|ship| ship.selected && ship.is_attacker == session.player_is_attacker)
        .map(|ship| ship.navigation_route.as_slice())
        .chain(
            session
                .fighters
                .iter()
                .filter(|fighter| fighter.selected && fighter.is_attacker == session.player_is_attacker)
                .map(|fighter| fighter.navigation_route.as_slice()),
        )
        .collect()
}

/// `FUN_005ca330`: a point already on the route is dropped (`FUN_005a8ed0`),
/// another one added (`FUN_005a8f20`). Returns whether it dropped one.
fn toggle_route_point(route: &mut Vec<TacticalWorldPosition>, point: TacticalWorldPosition) -> bool {
    if let Some(at) = route.iter().position(|each| *each == point) {
        route.remove(at);
        true
    } else {
        route.push(point);
        false
    }
}

/// A navigation point's material by its grid layer (`FUN_0059f800` stores the
/// layer, y index + 1, at `+0x54`, which the marker keeps at `+0x3c`):
/// `FUN_005d8c30` colours the three layer meshes (1, 0, 0), (1, 1, 1) and
/// (0, 0.541, 1), and A0 VM 2026-10-27 shows the top layer white, the middle
/// one red and the bottom one blue.
fn navigation_layer_tint(y: f32) -> Vec3 {
    if y > 0.0 {
        vec3(1.0, 1.0, 1.0)
    } else if y < 0.0 {
        vec3(0.0, 0.541, 1.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    }
}

/// `FUN_005d8c30`'s yellow (1, 1, 0) and magenta (1, 0, 1) meshes.
const NAVIGATION_ROUTE_NEXT_TINT: Vec3 = vec3(1.0, 1.0, 0.0);
const NAVIGATION_ROUTE_LATER_TINT: Vec3 = vec3(1.0, 0.0, 1.0);

/// A point's marker material (`FUN_0059f560`, first match wins): `+0x20`
/// yellow, which `FUN_005a0840` sets on the first point of the selected
/// unit's route; `+0x2c` magenta, set on its later points, and on the
/// selected point (A0 VM 2026-10-27 shows it purple, the magenta face in
/// ambient light); otherwise the layer's colour. A0 TAC-04-C002 turns the
/// first point of a route yellow, TAC-04-C003 its second one magenta.
fn navigation_point_tint(
    position: TacticalWorldPosition,
    routes: &[&[TacticalWorldPosition]],
    selected: bool,
) -> Vec3 {
    if routes.iter().any(|route| route.first() == Some(&position)) {
        NAVIGATION_ROUTE_NEXT_TINT
    } else if selected || routes.iter().any(|route| route.contains(&position)) {
        NAVIGATION_ROUTE_LATER_TINT
    } else {
        navigation_layer_tint(position.y)
    }
}

/// The markers of the shown points (`FUN_0059fb30` -> `FUN_005c1e80`), each
/// a lit tetrahedron (`FUN_005d90b0`) at the point.
fn tactical_navigation_markers(
    session: &BattleSession,
    projections: &[TacticalNavigationProjection],
    selected_point: Option<TacticalWorldPosition>,
) -> Vec<(Vec3, Vec3)> {
    let routes = selected_routes(session);
    projections
        .iter()
        .map(|projection| {
            (
                projection.position.rendered(),
                navigation_point_tint(
                    projection.position,
                    &routes,
                    selected_point == Some(projection.position),
                ),
            )
        })
        .collect()
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
    let thickness = canvas_scale.max(1.0).round();
    let (origin, max) = (
        snap_line_to_pixels(origin, thickness),
        snap_line_to_pixels(max, thickness),
    );
    let size = max - origin;
    let corner = (size.min_element() * 0.28).clamp(3.0 * canvas_scale, 8.0 * canvas_scale);
    for (x, x_direction) in [(origin.x, 1.0), (origin.x + size.x, -1.0)] {
        for (y, y_direction) in [(origin.y, 1.0), (origin.y + size.y, -1.0)] {
            draw_line(x, y, x + corner * x_direction, y, thickness, color);
            draw_line(x, y, x, y + corner * y_direction, thickness, color);
        }
    }
}

/// Centre a line of `thickness` on whole pixels: an odd width sits on a pixel
/// centre, an even one on a pixel edge, so it is drawn without blending into
/// the neighbouring rows.
fn snap_line_to_pixels(point: macroquad::math::Vec2, thickness: f32) -> macroquad::math::Vec2 {
    if (thickness.round() as i32) % 2 == 1 {
        point.floor() + macroquad::math::Vec2::splat(0.5)
    } else {
        point.round()
    }
}

/// Handle mouse input during the combat phase.
///
/// - Left-click: select a player ship.
/// - Right-click on friendly: issue the original order-1 Escort command.
/// - Right-click on enemy: issue focus-fire order to all selected player ships.
fn handle_combat_input(
    session: &mut BattleSession,
    death_star_targeting: &mut bool,
    selected_navigation_point: &mut Option<TacticalWorldPosition>,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    aperture: NativeRect,
    projections: &HashMap<usize, TacticalScreenProjection>,
    fighter_projections: &HashMap<usize, TacticalScreenProjection>,
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
            click_navigation_point(session, navigation, true);
            return;
        }
    }

    if inside_battle && is_mouse_button_pressed(MouseButton::Left) {
        *selected_navigation_point = None;
    }

    // Left-click: select a ship of either side.
    if inside_battle && is_mouse_button_pressed(MouseButton::Left) {
        let mut hit = None;
        for (i, ship) in session.ships.iter().enumerate() {
            if !ship.alive || ship.retreating {
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

        // FUN_005c2060 (`0x201`): a click on a unit posts it with Ctrl to
        // the selection state's slot `+0xc` (`FUN_00595b70(unit, 1)`), or
        // without Ctrl clears the selection first; a click on nothing
        // clears it unless Ctrl is held.
        if let Some(idx) = hit {
            if control {
                let outcome = ctrl_click_ship(session, idx);
                macroquad::logging::info!(
                    "[tactical_3d] ctrl_click object_id={} outcome={outcome}",
                    idx + 1,
                );
                if matches!(outcome, "toggled" | "selected" | "joined_task_force" | "left_task_force") {
                    macroquad::logging::info!(
                        "[tactical_3d] selection object_id={} source_projection={}",
                        idx + 1,
                        projections.contains_key(&idx)
                    );
                }
                return;
            }
            select_clicked_ship(session, idx, control);
            macroquad::logging::info!(
                "[tactical_3d] selection object_id={} source_projection={}",
                idx + 1,
                projections.contains_key(&idx)
            );
        } else if let Some(navigation) = navigation_hit.filter(|_| !control) {
            // A left click on a navigation point clears the selection
            // (`FUN_00595b70(0, 0)`) and selects the point (`0x005a0e10`
            // builds `0x66bb58`, `FUN_005a0e70`). port: a ship under the
            // pointer is taken first, as the port has no pick depth.
            select_navigation_point(session, selected_navigation_point, navigation.position);
            macroquad::logging::info!(
                "[tactical_navigation] event=point_selected set={} point={}",
                navigation.set + 1,
                navigation.point + 1,
            );
        } else if !control {
            // Clicked empty space — deselect all.
            for s in &mut session.ships {
                s.selected = false;
            }
            session.selected_ship = None;
            session.selected_fighter_group = None;
            session.task_force_selection = None;
            session.selection_state_open = false;
            for fighter in &mut session.fighters {
                fighter.selected = false;
            }
        }
    }

    // Right-click: assign an armed Death Star superlaser or issue focus fire.
    if inside_battle && is_mouse_button_pressed(MouseButton::Right) {
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
        // port: a unit under the pointer is taken before a navigation point,
        // as the port has no pick depth (`FUN_005c2d20` takes the first
        // pickable visual).
        let squadron_under_pointer = enemy_squadron_hit(
            session,
            fighter_projections.iter().map(|(&index, projection)| {
                (index, projection.contains(screen_pointer, projected_hit_radius))
            }),
        );
        if let Some(navigation) = navigation_hit
            .filter(|_| no_unit_under_pointer(target_hit, friendly_hit, squadron_under_pointer))
        {
            click_navigation_point(session, navigation, control);
            return;
        }
        if let Some(target_idx) = friendly_hit.filter(|_| control) {
            // FUN_005c2060 (`0x204`) posts the unit with Ctrl, as a Ctrl
            // left click does.
            let outcome = ctrl_click_ship(session, target_idx);
            macroquad::logging::info!(
                "[tactical_3d] ctrl_click object_id={} outcome={outcome}",
                target_idx + 1,
            );
            return;
        }
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

        // An enemy squadron is a target too: the state's slot `+0xc` takes
        // any unit (`0x005a1af0`, `0x005a4bd0`).
        let squadron_hit = enemy_squadron_hit(
            session,
            fighter_projections.iter().map(|(&index, projection)| {
                (index, projection.contains(screen_pointer, projected_hit_radius))
            }),
        );
        let unit_target = target_hit
            .map(TacticalAttackTarget::CapitalShip)
            .or_else(|| squadron_hit.map(TacticalAttackTarget::FighterGroup));
        if session.selected_fighter_group.is_some() {
            if let Some(target) = unit_target {
                let members = assign_selected_fighter_group_target(session, target, control);
                macroquad::logging::info!(
                    "[tactical_navigation] event=fighter_target_assign append={} target={:?} members={}",
                    control,
                    target,
                    members,
                );
                return;
            }
        } else if let (None, Some(fighter_target)) = (target_hit, squadron_hit) {
            assign_selected_focus_target(
                session,
                TacticalAttackTarget::FighterGroup(fighter_target),
                control,
            );
            macroquad::logging::info!(
                "[tactical_navigation] event=target_assign status=assigned append={} target_fighter={}",
                control,
                fighter_target + 1,
            );
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
        }
        // FUN_005c2060 (`0x204`): a right click on nothing posts nothing.
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
            session.task_force_selection = None;
        } else {
            // Deselect.
            for s in &mut session.ships {
                s.selected = false;
            }
            session.selected_ship = None;
            session.task_force_selection = None;
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
#[path = "tactical_view_tests.rs"]
mod tests;
