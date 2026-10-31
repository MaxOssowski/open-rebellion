//! Original bitmap-driven unified Game Options surface.

use macroquad::prelude::*;

use crate::audio::AudioVolumeState;
use crate::bmp_cache::{resources, BmpCache, DllSource};
use crate::panels::SaveSlotInfo;

const LOGICAL_WIDTH: f32 = 640.0;
const LOGICAL_HEIGHT: f32 = 480.0;
/// The port's build, shown where the original showed "1.01.00" and its
/// language.
const PORT_VERSION: &str = "1.0.0 Open Rebellion";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOptionsOrigin {
    ShuttleCockpit,
    CommandCenter,
    TacticalBattle,
}

impl GameOptionsOrigin {
    const fn has_live_campaign(self) -> bool {
        !matches!(self, Self::ShuttleCockpit)
    }

    /// `FUN_00407180` never disables Restart; from the shuttle cockpit it
    /// shows 10035 and returns there at once (A0 main-menu-states/12).
    /// port: a battle's Restart stays off until an A0 capture shows it.
    const fn can_restart(self) -> bool {
        !matches!(self, Self::TacticalBattle)
    }

    const fn tactical_toggles_enabled(self) -> bool {
        !matches!(self, Self::TacticalBattle)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameOptionsAction {
    None,
    Return,
    Restart,
    Exit,
    Save { slot: usize, name: String },
    Load { slot: usize },
    Delete { slot: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameOptionsControl {
    Save(usize),
    Load(usize),
    Music,
    MusicVolume,
    SoundVolume,
    Tactical(usize),
    Restart,
    Return,
    Exit,
}

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
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

// The controls' rectangles are FUN_00407180's literal arguments; A0
// 200-options matches the Save, Load, Restart, Return, Exit and Play Music
// bitmaps there pixel for pixel.
const MUSIC_RECT: NativeRect = NativeRect::new(352.0, 76.0, 19.0, 35.0);
const MUSIC_SLIDER_RECT: NativeRect = NativeRect::new(392.0, 130.0, 201.0, 54.0);
const SOUND_SLIDER_RECT: NativeRect = NativeRect::new(392.0, 190.0, 201.0, 54.0);
const TACTICAL_TOGGLE_RECTS: [NativeRect; 5] = [
    NativeRect::new(357.0, 311.0, 35.0, 22.0),
    NativeRect::new(357.0, 337.0, 35.0, 22.0),
    NativeRect::new(357.0, 365.0, 35.0, 22.0),
    NativeRect::new(357.0, 392.0, 35.0, 22.0),
    NativeRect::new(357.0, 419.0, 35.0, 22.0),
];
const RESTART_RECT: NativeRect = NativeRect::new(76.0, 381.0, 42.0, 42.0);
const RETURN_RECT: NativeRect = NativeRect::new(162.0, 382.0, 42.0, 42.0);
const EXIT_RECT: NativeRect = NativeRect::new(248.0, 381.0, 42.0, 42.0);

#[derive(Debug, Clone, Copy)]
struct OptionsCanvas {
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl OptionsCanvas {
    fn new(width: f32, height: f32) -> Self {
        let scale = (width / LOGICAL_WIDTH).min(height / LOGICAL_HEIGHT);
        Self {
            scale,
            offset_x: (width - LOGICAL_WIDTH * scale) * 0.5,
            offset_y: (height - LOGICAL_HEIGHT * scale) * 0.5,
        }
    }

    fn point(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.offset_x + x * self.scale,
            self.offset_y + y * self.scale,
        )
    }

    fn logical_pointer(self) -> (f32, f32) {
        let (x, y) = mouse_position();
        (
            (x - self.offset_x) / self.scale,
            (y - self.offset_y) / self.scale,
        )
    }
}

#[derive(Debug, Clone)]
pub struct GameOptionsState {
    pub pending: Option<GameOptionsAction>,
    /// The one-button message after a save (`FUN_00408240`): `Some(true)`
    /// "Selected Game Saved Successfully", `Some(false)` "Selected Game
    /// Failed To Save".
    pub saved_notice: Option<bool>,
    /// A save succeeded since the window opened (`FUN_00408240` sets the
    /// slot's flag at `+0xac + 4k`); Restart then asks nothing.
    pub saved_this_visit: bool,
    pub occupied: [bool; 10],
    pub names: [String; 6],
    pub suspended: bool,
    pub error: Option<String>,
    origin: GameOptionsOrigin,
    pressed: Option<GameOptionsControl>,
    pub show_starfield: bool,
    pub show_planet: bool,
    pub show_pyrotechnics: bool,
    pub high_detail_models: bool,
    pub display_holocube: bool,
    /// The player's side, which picks the Return button's art in the
    /// command center.
    pub player_is_alliance: bool,
}

impl Default for GameOptionsState {
    fn default() -> Self {
        Self {
            pending: None,
            saved_notice: None,
            saved_this_visit: false,
            occupied: [false; 10],
            names: std::array::from_fn(|_| String::new()),
            suspended: false,
            error: None,
            origin: GameOptionsOrigin::ShuttleCockpit,
            pressed: None,
            show_starfield: true,
            show_planet: true,
            show_pyrotechnics: true,
            high_detail_models: true,
            display_holocube: true,
            player_is_alliance: true,
        }
    }
}

impl GameOptionsState {
    pub fn refresh_saves(&mut self, saves: &[SaveSlotInfo]) {
        self.occupied.fill(false);
        // An empty slot's name box is empty (FUN_00407180).
        self.names = std::array::from_fn(|_| String::new());
        for save in saves {
            if let Some(occupied) = self.occupied.get_mut(save.slot) {
                *occupied = true;
            }
            if let Some(name) = self.names.get_mut(save.slot) {
                name.clone_from(&save.name);
            }
        }
    }

    fn enabled(&self, action: &GameOptionsAction) -> bool {
        match action {
            // FUN_00406b80 commands 0x6f..0x74 save whatever the name box
            // holds (FUN_00408000), an empty name included.
            GameOptionsAction::Save { slot, .. } => {
                self.origin == GameOptionsOrigin::CommandCenter && *slot < self.occupied.len()
            }
            GameOptionsAction::Load { slot } | GameOptionsAction::Delete { slot } => {
                self.origin != GameOptionsOrigin::TacticalBattle
                    && self.occupied.get(*slot).copied().unwrap_or(false)
            }
            GameOptionsAction::Restart => self.origin.can_restart(),
            GameOptionsAction::Return | GameOptionsAction::Exit => true,
            GameOptionsAction::None => false,
        }
    }

    /// Whether a question or a message holds the window.
    #[must_use]
    pub const fn modal(&self) -> bool {
        self.pending.is_some() || self.saved_notice.is_some()
    }

    pub fn request(&mut self, action: GameOptionsAction) -> GameOptionsAction {
        if self.suspended || self.modal() || !self.enabled(&action) {
            return GameOptionsAction::None;
        }
        let confirm = match &action {
            GameOptionsAction::Save { slot, .. } => self.occupied[*slot],
            GameOptionsAction::Load { .. } => self.origin.has_live_campaign(),
            // FUN_00406b80 command 0x91: a game in progress (+0xa8) with
            // no save made in this visit asks 0x476 (FUN_00408170); else
            // Restart goes straight to the shuttle cockpit.
            GameOptionsAction::Restart => self.origin.has_live_campaign() && !self.saved_this_visit,
            GameOptionsAction::Exit | GameOptionsAction::Delete { .. } => true,
            _ => false,
        };
        self.error = None;
        if confirm {
            self.pending = Some(action);
            GameOptionsAction::None
        } else {
            action
        }
    }

    pub fn confirm(&mut self, accepted: bool) -> GameOptionsAction {
        self.pending
            .take()
            .filter(|action| accepted && self.enabled(action))
            .unwrap_or(GameOptionsAction::None)
    }

    pub fn escape(&mut self) -> GameOptionsAction {
        // The message takes only Enter (Class_006589a8_vt4c); a question
        // takes Escape as No (Class_00658950_vt4c).
        if self.saved_notice.is_some() || self.pending.take().is_some() {
            GameOptionsAction::None
        } else {
            GameOptionsAction::Return
        }
    }

    #[must_use]
    pub fn new(origin: GameOptionsOrigin) -> Self {
        Self {
            origin,
            ..Self::default()
        }
    }

    pub fn set_origin(&mut self, origin: GameOptionsOrigin) {
        self.origin = origin;
        self.pressed = None;
        self.pending = None;
        self.saved_notice = None;
        self.saved_this_visit = false;
        self.suspended = false;
        self.error = None;
    }

    #[must_use]
    pub const fn origin(&self) -> GameOptionsOrigin {
        self.origin
    }

    #[must_use]
    pub const fn tactical_flags(&self) -> [bool; 5] {
        [
            self.show_starfield,
            self.show_planet,
            self.show_pyrotechnics,
            self.high_detail_models,
            self.display_holocube,
        ]
    }
}

/// A question's two REBDLOG strings (`FUN_00408170`): Restart 0x121a and
/// 0x121b, Load 0x121c and 0x121d, Save 0x121e and 0x121f; Exit asks
/// 0x1202 alone (`FUN_00401110(0x470, 0, 0x1202)`, the quit question).
/// port: Delete has no original question.
fn confirmation_strings(action: &GameOptionsAction) -> (Option<String>, String) {
    let dialog = |id: u16, shipped: &str| {
        crate::win_tooltip::module_string(crate::win_tooltip::StringModule::Dialog, id)
            .unwrap_or_else(|| shipped.to_owned())
    };
    match action {
        GameOptionsAction::Restart => (
            Some(dialog(
                0x121a,
                "Returning to the shuttle cockpit will cause unsaved changes to be lost",
            )),
            dialog(0x121b, "Return without saving?"),
        ),
        GameOptionsAction::Load { .. } => (
            Some(dialog(
                0x121c,
                "Loading the selected game will destroy unsaved changes",
            )),
            dialog(0x121d, "Load without saving?"),
        ),
        GameOptionsAction::Save { .. } => (
            Some(dialog(
                0x121e,
                "Saving the selected game will destroy a previously saved game",
            )),
            dialog(0x121f, "Save anyway?"),
        ),
        GameOptionsAction::Exit => (None, dialog(0x1202, "Are you sure you want to quit?")),
        _ => (None, "Delete the selected saved game?".to_owned()),
    }
}

/// The Return button's normal, pressed and disabled art (FUN_00407180,
/// switch on the window's mode at +0x9c): from the galaxy (mode 8) the
/// Empire's frame 10023..10025 unless the player is the Alliance
/// (FUN_00401060 == 1); from a battle (mode 9) the holocube 10026..10028;
/// otherwise the Alliance frame 10020..10022. A0 200-options (an Empire
/// game, from the galaxy) shows 10023 at (162, 382). Modes 4 and 0xc
/// (the multiplayer console 10029..10031) are out of the port's scope.
fn return_art(origin: GameOptionsOrigin, player_is_alliance: bool) -> [u32; 3] {
    match origin {
        GameOptionsOrigin::CommandCenter if !player_is_alliance => [10023, 10024, 10025],
        GameOptionsOrigin::TacticalBattle => [10026, 10027, 10028],
        _ => [
            resources::common::BTN_RETURN_COMMAND_CENTER_NORMAL,
            resources::common::BTN_RETURN_COMMAND_CENTER_PRESSED,
            resources::common::BTN_RETURN_COMMAND_CENTER_DISABLED,
        ],
    }
}

fn save_rect(slot: usize) -> NativeRect {
    NativeRect::new(34.0, 81.0 + slot as f32 * 42.0, 42.0, 20.0)
}

fn load_rect(slot: usize) -> NativeRect {
    NativeRect::new(287.0, 81.0 + slot as f32 * 42.0, 41.0, 20.0)
}

impl GameOptionsControl {
    /// The control's tooltip (`FUN_00407180`, TEXTCOMM): Save Game 0x1010,
    /// Load Game 0x1011, Music Volume 0x1012, Sound Effects Volume 0x1013,
    /// "Restart the Game " 0x1014, Return to the Command Center 0x1015 and
    /// Exit 0x1018 (A0 tooltip/201..207). The music and tactical toggles
    /// have none (208, 209).
    const fn tooltip_string(self) -> Option<u16> {
        match self {
            Self::Save(_) => Some(0x1010),
            Self::Load(_) => Some(0x1011),
            Self::MusicVolume => Some(0x1012),
            Self::SoundVolume => Some(0x1013),
            Self::Restart => Some(0x1014),
            Self::Return => Some(0x1015),
            Self::Exit => Some(0x1018),
            Self::Music | Self::Tactical(_) => None,
        }
    }
}

fn control_at(origin: GameOptionsOrigin, x: f32, y: f32) -> Option<GameOptionsControl> {
    for slot in 0..6 {
        if origin == GameOptionsOrigin::CommandCenter && save_rect(slot).contains(x, y) {
            return Some(GameOptionsControl::Save(slot));
        }
        if origin != GameOptionsOrigin::TacticalBattle && load_rect(slot).contains(x, y) {
            return Some(GameOptionsControl::Load(slot));
        }
    }
    if MUSIC_RECT.contains(x, y) {
        return Some(GameOptionsControl::Music);
    }
    if MUSIC_SLIDER_RECT.contains(x, y) {
        return Some(GameOptionsControl::MusicVolume);
    }
    if SOUND_SLIDER_RECT.contains(x, y) {
        return Some(GameOptionsControl::SoundVolume);
    }
    for (index, rect) in TACTICAL_TOGGLE_RECTS.into_iter().enumerate() {
        if origin.tactical_toggles_enabled() && rect.contains(x, y) {
            return Some(GameOptionsControl::Tactical(index));
        }
    }
    if origin.can_restart() && RESTART_RECT.contains(x, y) {
        return Some(GameOptionsControl::Restart);
    }
    if origin.has_live_campaign() && RETURN_RECT.contains(x, y) {
        return Some(GameOptionsControl::Return);
    }
    EXIT_RECT.contains(x, y).then_some(GameOptionsControl::Exit)
}

fn masked_control_at(
    cache: &mut BmpCache,
    origin: GameOptionsOrigin,
    x: f32,
    y: f32,
) -> Option<GameOptionsControl> {
    let control = control_at(origin, x, y)?;
    let (id, rect) = match control {
        GameOptionsControl::Save(slot) => (resources::common::OPTIONS_SAVE_NORMAL, save_rect(slot)),
        GameOptionsControl::Load(slot) => (resources::common::OPTIONS_LOAD_NORMAL, load_rect(slot)),
        GameOptionsControl::Music => (resources::common::OPTIONS_MUSIC_NORMAL, MUSIC_RECT),
        GameOptionsControl::MusicVolume => {
            (resources::common::OPTIONS_VOLUME_RAIL, MUSIC_SLIDER_RECT)
        }
        GameOptionsControl::SoundVolume => (
            resources::common::OPTIONS_VOLUME_RAIL_ALT,
            SOUND_SLIDER_RECT,
        ),
        GameOptionsControl::Tactical(index) => (
            resources::common::OPTIONS_TOGGLE_NORMAL,
            TACTICAL_TOGGLE_RECTS[index],
        ),
        GameOptionsControl::Restart => (resources::common::BTN_RESTART_GAME_NORMAL, RESTART_RECT),
        GameOptionsControl::Return => (return_art(origin, true)[0], RETURN_RECT),
        GameOptionsControl::Exit => (resources::common::BTN_EXIT_GAME_NORMAL, EXIT_RECT),
    };
    cache
        .is_resource_hit(
            DllSource::Common,
            id,
            (x - rect.x) as usize,
            (y - rect.y) as usize,
        )
        .then_some(control)
}

/// The background and the slider tracks, whose palette entry 254 (cyan in
/// their DIBs) the screen shows as white (A0 200-options: all 1328 such
/// pixels of COMMON 20002 and 664 of each track).
fn draw_screen_bitmap(
    cache: &mut BmpCache,
    canvas: OptionsCanvas,
    resource_id: u32,
    x: f32,
    y: f32,
) {
    let Some(texture) =
        cache.get_macroquad_with_entry(DllSource::Common, resource_id, 254, [255, 255, 255])
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

fn draw_bitmap(cache: &mut BmpCache, canvas: OptionsCanvas, resource_id: u32, x: f32, y: f32) {
    let Some(texture) = cache.get_macroquad_original(DllSource::Common, resource_id) else {
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

thread_local! {
    static ARIAL: std::cell::OnceCell<Option<Font>> = const { std::cell::OnceCell::new() };
    static ARIAL_BOLD: std::cell::OnceCell<Option<Font>> = const { std::cell::OnceCell::new() };
}

/// The Arial-metric face of the game font table (`FUN_0060eed0`).
fn arial(bold: bool) -> Option<Font> {
    let cell = if bold { &ARIAL_BOLD } else { &ARIAL };
    cell.with(|cell| {
        cell.get_or_init(|| {
            load_ttf_font_from_bytes(if bold {
                crate::theme::ARIAL_BOLD
            } else {
                crate::theme::ARIAL_REGULAR
            })
            .ok()
        })
        .clone()
    })
}

/// How a text object lays its line in its rectangle (`DrawTextA` formats).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextFormat {
    /// `0x25`: centred both ways, one line.
    Centred,
    /// `0x24`: left, centred vertically, one line.
    LeftMiddle,
    /// `0x10`: left, from the top.
    TopLeft,
}

/// Draw `text` in game font `font` (`theme::original_font_cell`) in the
/// logical rectangle `(x, y, w, h)` as GDI's `DrawTextA` would with
/// `format`, with an optional one-pixel shadow down and to the right,
/// clipped to the rectangle when `clip`.
#[expect(
    clippy::too_many_arguments,
    reason = "A text's own parameters: text, rect, font, colour, format, shadow and clip."
)]
fn draw_original_text(
    canvas: OptionsCanvas,
    text: &str,
    (x, y, w, h): (f32, f32, f32, f32),
    font: u16,
    color: Color,
    format: TextFormat,
    shadow: Option<Color>,
    clip: bool,
) {
    let (_, bold) = crate::theme::original_font_cell(font);
    // GDI's size for the font, matched in Liberation's widths
    // (theme::gdi_font).
    let em = crate::theme::gdi_font(font, 1.0).size;
    let face = arial(bold);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "A small font size."
    )]
    let size = (em * canvas.scale).round().max(1.0) as u16;
    // GDI's pens and width (Arial's advances and kerning), else the face's.
    let pens = crate::theme::gdi_pen_positions(font, text);
    let width = pens.as_ref().map_or_else(
        || measure_text(text, face.as_ref(), size, 1.0).width / canvas.scale,
        |(_, width)| *width,
    );
    use crate::theme::{GdiAlign, GdiVAlign};
    let (left, baseline) = crate::theme::gdi_text_origin(
        (x, y, w, h),
        font,
        width,
        match format {
            TextFormat::Centred => (GdiAlign::Center, GdiVAlign::VCenter),
            TextFormat::LeftMiddle => (GdiAlign::Left, GdiVAlign::VCenter),
            TextFormat::TopLeft => (GdiAlign::Left, GdiVAlign::Top),
        },
    );
    let paint = |dx: f32, color: Color| {
        let params = TextParams {
            font: face.as_ref(),
            font_size: size,
            font_scale: 1.0,
            color,
            ..Default::default()
        };
        if let Some((pens, _)) = &pens {
            let mut buffer = [0u8; 4];
            for (ch, pen) in text.chars().zip(pens) {
                let (sx, sy) = canvas.point(left + pen + dx, baseline + dx);
                draw_text_ex(
                    ch.encode_utf8(&mut buffer),
                    sx.round(),
                    sy.round(),
                    params.clone(),
                );
            }
        } else {
            let (sx, sy) = canvas.point(left + dx, baseline + dx);
            draw_text_ex(text, sx.round(), sy.round(), params);
        }
    };
    // A heading is DrawTextA without DT_NOCLIP (FUN_005ff6b0, format 0x25)
    // and clips to its rectangle; the text objects add DT_NOCLIP
    // (FUN_00601ce0), so their descenders reach below their 16-pixel rows
    // (A0 200-options: "Show Pyrotechnics" to y 384).
    let (clip_x, clip_y) = canvas.point(x, y);
    #[expect(clippy::cast_possible_truncation, reason = "Screen pixels.")]
    let clip = clip.then(|| {
        (
            clip_x.round() as i32,
            clip_y.round() as i32,
            (w * canvas.scale).round() as i32,
            (h * canvas.scale).round() as i32,
        )
    });
    // SAFETY: macroquad exposes its immediate drawing state through this
    // API; the clip is cleared before returning.
    unsafe {
        get_internal_gl().quad_gl.scissor(clip);
    }
    if let Some(shadow) = shadow {
        paint(1.0, shadow);
    }
    paint(0.0, color);
    // SAFETY: as above.
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}

/// `COLORREF` 0x200ff00, bright green: the headings and a set option.
const OPTION_ON: Color = Color::new(0.0, 1.0, 0.0, 1.0);
/// `COLORREF` 0x2008000, dark green: an option that is off.
const OPTION_OFF: Color = Color::new(0.0, 128.0 / 255.0, 0.0, 1.0);
/// `COLORREF` 0x2505050, grey: the display options during a battle
/// (`FUN_00408bc0`, states 20..23).
const OPTION_DISABLED: Color = Color::new(80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0);

/// A TEXTCOMM string, as shipped when the table is not loaded.
fn textcomm(id: u16, shipped: &str) -> String {
    crate::win_tooltip::module_string(crate::win_tooltip::StringModule::Common, id)
        .unwrap_or_else(|| shipped.to_owned())
}

/// A volume slider (`FUN_00604570`, painted by `FUN_00604a50`): the whole
/// track at the slider's origin, then the thumb (COMMON 10054, 11 by 47)
/// centred on `floor(201 * value / 100)` at y + 4, clipped to the track's
/// 201 by 54. A0 200-options shows the track's palette entry 254 (cyan in
/// the DIB) as white on this screen.
fn draw_volume(cache: &mut BmpCache, canvas: OptionsCanvas, y: f32, value: f32, alternate: bool) {
    let track = if alternate {
        resources::common::OPTIONS_VOLUME_RAIL_ALT
    } else {
        resources::common::OPTIONS_VOLUME_RAIL
    };
    draw_screen_bitmap(cache, canvas, track, SLIDER_X, y);
    let Some(thumb) =
        cache.get_macroquad_original(DllSource::Common, resources::common::OPTIONS_VOLUME_HANDLE)
    else {
        return;
    };
    let left = slider_thumb_centre(value) - (thumb.width() / 2.0).floor();
    let visible = (SLIDER_WIDTH - left).min(thumb.width()).max(0.0);
    let (screen_x, screen_y) = canvas.point(SLIDER_X + left, y + 4.0);
    draw_texture_ex(
        thumb,
        screen_x,
        screen_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(visible * canvas.scale, thumb.height() * canvas.scale)),
            source: Some(Rect::new(0.0, 0.0, visible, thumb.height())),
            ..Default::default()
        },
    );
}

/// The slider's left edge (`FUN_00407180`: x 0x188) and client width.
const SLIDER_X: f32 = 392.0;
const SLIDER_WIDTH: f32 = 201.0;

/// The thumb's centre in slider pixels for a volume of `value` (0..1):
/// `floor(201 * v / 100)` for the slider's whole-percent `v`.
fn slider_thumb_centre(value: f32) -> f32 {
    (SLIDER_WIDTH * (value.clamp(0.0, 1.0) * 100.0).round() / 100.0).floor()
}

/// A drag on a slider (`FUN_00604950`): only a pointer strictly between
/// x 5 and 196 in the slider moves the thumb, to `floor(x * 100 / 201)`
/// percent, so a drag gives 2..97 and never 0 or 100.
fn set_slider(audio: &mut AudioVolumeState, control: GameOptionsControl, logical_x: f32) {
    let x = (logical_x - SLIDER_X).floor();
    if x <= 5.0 || x >= SLIDER_WIDTH - 5.0 {
        return;
    }
    let value = (x * 100.0 / SLIDER_WIDTH).floor() / 100.0;
    match control {
        GameOptionsControl::MusicVolume => audio.music_volume = value,
        GameOptionsControl::SoundVolume => audio.sfx_volume = value,
        _ => {}
    }
    audio.dirty = true;
}

fn toggle_tactical(state: &mut GameOptionsState, index: usize) {
    match index {
        0 => state.show_starfield = !state.show_starfield,
        1 => state.show_planet = !state.show_planet,
        2 => state.show_pyrotechnics = !state.show_pyrotechnics,
        3 => state.high_detail_models = !state.high_detail_models,
        4 => state.display_holocube = !state.display_holocube,
        _ => {}
    }
}

fn activate(
    state: &mut GameOptionsState,
    control: GameOptionsControl,
    saves: &[SaveSlotInfo],
    audio: &mut AudioVolumeState,
) -> GameOptionsAction {
    match control {
        GameOptionsControl::Music => {
            audio.toggle_music();
            macroquad::logging::info!(
                "[game_options] control=music enabled={}",
                audio.music_enabled()
            );
        }
        GameOptionsControl::Tactical(index) => {
            toggle_tactical(state, index);
            macroquad::logging::info!(
                "[game_options] control=tactical index={} enabled={}",
                index,
                state.tactical_flags()[index]
            );
        }
        GameOptionsControl::Save(slot) => {
            let name = state.names[slot].clone();
            return state.request(GameOptionsAction::Save { slot, name });
        }
        GameOptionsControl::Load(slot) => {
            if saves.iter().any(|save| save.slot == slot) {
                return state.request(GameOptionsAction::Load { slot });
            }
        }
        GameOptionsControl::Restart => return state.request(GameOptionsAction::Restart),
        GameOptionsControl::Return => return state.request(GameOptionsAction::Return),
        GameOptionsControl::Exit => return state.request(GameOptionsAction::Exit),
        GameOptionsControl::MusicVolume | GameOptionsControl::SoundVolume => {}
    }
    GameOptionsAction::None
}

/// Draw the original 640×480 Game Options screen and dispatch its controls.
pub fn draw_game_options(
    state: &mut GameOptionsState,
    cache: &mut BmpCache,
    saves: &[SaveSlotInfo],
    audio: &mut AudioVolumeState,
) -> GameOptionsAction {
    let canvas = OptionsCanvas::new(screen_width(), screen_height());
    clear_background(BLACK);
    draw_screen_bitmap(cache, canvas, resources::common::GAME_OPTIONS_BG, 0.0, 0.0);

    // FUN_004084b0 draws the headings into the window's background with
    // FUN_005ff6b0: TEXTCOMM 0x1028, 0x1026, 0x1027, font 7 (18 bold),
    // 0x200ff00, format 0x25.
    for (id, shipped, rect) in [
        (0x1028, "Saved Games", (58.0, 37.0, 241.0, 21.0)),
        (0x1026, "Sound Options", (362.0, 37.0, 241.0, 21.0)),
        (
            0x1027,
            "Tactical Display Options",
            (363.0, 273.0, 239.0, 21.0),
        ),
    ] {
        let text = textcomm(id, shipped);
        draw_original_text(
            canvas,
            &text,
            rect,
            7,
            OPTION_ON,
            TextFormat::Centred,
            None,
            true,
        );
    }
    // Its text objects (FUN_00601620): each label and its On/Off in font 7,
    // bright green when the option is set, dark green when not; "On" and
    // "Off" are TEXTCOMM 0x1031 / 0x1032, left-aligned.
    let switch = |set: bool| if set { OPTION_ON } else { OPTION_OFF };
    let on_off = |set: bool| {
        if set {
            textcomm(0x1031, "On")
        } else {
            textcomm(0x1032, "Off")
        }
    };
    let music = audio.music_enabled();
    let text = textcomm(0x1025, "Play Music");
    draw_original_text(
        canvas,
        &text,
        (376.0, 79.0, 183.0, 33.0),
        7,
        switch(music),
        TextFormat::LeftMiddle,
        None,
        false,
    );
    draw_original_text(
        canvas,
        &on_off(music),
        (581.0, 79.0, 31.0, 33.0),
        7,
        switch(music),
        TextFormat::LeftMiddle,
        None,
        false,
    );
    for (index, (id, shipped)) in [
        (0x101f, "Show Starfield"),
        (0x1020, "Show Planet"),
        (0x1021, "Show Pyrotechnics"),
        (0x1022, "Use High Detail Models"),
        (0x1024, "Display Holocube"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 313.0 + index as f32 * 27.0;
        let set = state.tactical_flags()[index];
        let color = if state.origin.tactical_toggles_enabled() {
            switch(set)
        } else {
            OPTION_DISABLED
        };
        let text = textcomm(id, shipped);
        draw_original_text(
            canvas,
            &text,
            (394.0, y, 183.0, 16.0),
            7,
            color,
            TextFormat::TopLeft,
            None,
            false,
        );
        draw_original_text(
            canvas,
            &on_off(set),
            (577.0, y, 31.0, 16.0),
            7,
            color,
            TextFormat::TopLeft,
            None,
            false,
        );
    }
    // The version label (+0x118): (25, 437), 310 by 18, font 10, black,
    // centred; TEXTCOMM 0x100c "Version: " and the build. port: the port's
    // own version stands where the original's "1.01.00" and its language
    // name stood.
    let version = format!("{}{}", textcomm(0x100c, "Version: "), PORT_VERSION);
    draw_original_text(
        canvas,
        &version,
        (25.0, 437.0, 310.0, 18.0),
        10,
        BLACK,
        TextFormat::Centred,
        None,
        false,
    );

    let pointer = canvas.logical_pointer();
    let held = state
        .pressed
        .filter(|_| is_mouse_button_down(MouseButton::Left));
    draw_bitmap(
        cache,
        canvas,
        // A set switch shows its pressed art (FUN_00603150(4, 0x2739)); A0
        // 200-options, music off, shows 10040.
        if !audio.backend_available {
            resources::common::OPTIONS_MUSIC_DISABLED
        } else if held == Some(GameOptionsControl::Music) || audio.music_enabled() {
            resources::common::OPTIONS_MUSIC_PRESSED
        } else {
            resources::common::OPTIONS_MUSIC_NORMAL
        },
        MUSIC_RECT.x,
        MUSIC_RECT.y,
    );
    draw_volume(
        cache,
        canvas,
        MUSIC_SLIDER_RECT.y,
        audio.music_volume,
        false,
    );
    draw_volume(cache, canvas, SOUND_SLIDER_RECT.y, audio.sfx_volume, true);

    for (index, rect) in TACTICAL_TOGGLE_RECTS.into_iter().enumerate() {
        // A set switch shows its pressed art (FUN_00603150(4, 0x273c)); A0
        // 200-options, all five on, shows 10044 in each.
        let resource = if !state.origin.tactical_toggles_enabled() {
            resources::common::OPTIONS_TOGGLE_DISABLED
        } else if held == Some(GameOptionsControl::Tactical(index)) || state.tactical_flags()[index]
        {
            resources::common::OPTIONS_TOGGLE_PRESSED
        } else {
            resources::common::OPTIONS_TOGGLE_NORMAL
        };
        draw_bitmap(cache, canvas, resource, rect.x, rect.y);
    }

    for slot in 0..6 {
        let occupied = saves.iter().find(|save| save.slot == slot);
        let save_resource = if state.origin != GameOptionsOrigin::CommandCenter {
            resources::common::OPTIONS_SAVE_DISABLED
        } else if held == Some(GameOptionsControl::Save(slot)) {
            resources::common::OPTIONS_SAVE_PRESSED
        } else {
            resources::common::OPTIONS_SAVE_NORMAL
        };
        let load_resource =
            if occupied.is_none() || state.origin == GameOptionsOrigin::TacticalBattle {
                resources::common::OPTIONS_LOAD_DISABLED
            } else if held == Some(GameOptionsControl::Load(slot)) {
                resources::common::OPTIONS_LOAD_PRESSED
            } else {
                resources::common::OPTIONS_LOAD_NORMAL
            };
        let save_bounds = save_rect(slot);
        let load_bounds = load_rect(slot);
        draw_bitmap(cache, canvas, save_resource, save_bounds.x, save_bounds.y);
        draw_bitmap(cache, canvas, load_resource, load_bounds.x, load_bounds.y);
        // FUN_00407180: the side's crest at (85, y + 2), COMMON 10056 for
        // the Alliance, 10055 for the Empire, only beside a name; an empty
        // slot shows none.
        if let Some(alliance) = occupied
            .filter(|save| !save.name.is_empty())
            .and_then(|save| save.player_is_alliance)
        {
            let crest = if alliance { 10056 } else { 10055 };
            draw_bitmap(cache, canvas, crest, 85.0, save_bounds.y + 2.0);
        }
        // The name box: the default font (16-pixel cell; A0 200-options's
        // "test" is 21 by 9, Arial at 13 ppem), white with a black
        // shadow (+0xe4 0x2ffffff, +0xe8 0x2000000), left, centred
        // vertically. In the command center the overlay's edit box shows it.
        if state.origin != GameOptionsOrigin::CommandCenter {
            if let Some(save) = occupied {
                draw_original_text(
                    canvas,
                    &save.name,
                    (119.0, save_bounds.y, 160.0, 20.0),
                    4,
                    WHITE,
                    TextFormat::LeftMiddle,
                    Some(BLACK),
                    true,
                );
            }
        }
    }

    draw_bitmap(
        cache,
        canvas,
        if !state.origin.can_restart() {
            resources::common::BTN_RESTART_GAME_DISABLED
        } else if held == Some(GameOptionsControl::Restart) {
            resources::common::BTN_RESTART_GAME_PRESSED
        } else {
            resources::common::BTN_RESTART_GAME_NORMAL
        },
        RESTART_RECT.x,
        RESTART_RECT.y,
    );
    draw_bitmap(
        cache,
        canvas,
        {
            let [normal, pressed, disabled] = return_art(state.origin, state.player_is_alliance);
            if !state.origin.has_live_campaign() {
                disabled
            } else if held == Some(GameOptionsControl::Return) {
                pressed
            } else {
                normal
            }
        },
        RETURN_RECT.x,
        RETURN_RECT.y,
    );
    draw_bitmap(
        cache,
        canvas,
        if held == Some(GameOptionsControl::Exit) {
            resources::common::BTN_EXIT_GAME_PRESSED
        } else {
            resources::common::BTN_EXIT_GAME_NORMAL
        },
        EXIT_RECT.x,
        EXIT_RECT.y,
    );

    if state.modal() || state.suspended {
        state.pressed = None;
        return GameOptionsAction::None;
    }
    if let Some(control @ (GameOptionsControl::MusicVolume | GameOptionsControl::SoundVolume)) =
        state.pressed
    {
        if is_mouse_button_down(MouseButton::Left) {
            set_slider(audio, control, pointer.0);
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        state.pressed = masked_control_at(cache, state.origin, pointer.0, pointer.1);
        if let Some(control @ (GameOptionsControl::MusicVolume | GameOptionsControl::SoundVolume)) =
            state.pressed
        {
            set_slider(audio, control, pointer.0);
        }
    }
    if is_mouse_button_released(MouseButton::Left) {
        let pressed = state.pressed.take();
        if pressed == masked_control_at(cache, state.origin, pointer.0, pointer.1) {
            if let Some(control) = pressed {
                return activate(state, control, saves, audio);
            }
        }
    }
    GameOptionsAction::None
}

/// Edit names and draw a confirmation above the macroquad options surface.
/// Missing original assets use visible standard buttons instead of an invisible modal.
pub fn draw_game_options_overlay(
    ctx: &egui_macroquad::egui::Context,
    cache: &mut BmpCache,
    state: &mut GameOptionsState,
) -> GameOptionsAction {
    use egui_macroquad::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};
    let screen = ctx.screen_rect();
    let canvas = OptionsCanvas::new(screen.width(), screen.height());
    let rect = |x, y, w, h| {
        let (x, y) = canvas.point(x, y);
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h) * canvas.scale)
    };
    let mut answer = None;
    // The shared tooltip (FUN_006008e0) over the options' 640 by 480, and
    // the tool under the pointer while no question shows.
    crate::win_tooltip::paint(
        ctx,
        Pos2::new(canvas.offset_x, canvas.offset_y),
        canvas.scale,
    );
    if !state.modal() && !state.suspended {
        if let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) {
            let x = (pointer.x - canvas.offset_x) / canvas.scale;
            let y = (pointer.y - canvas.offset_y) / canvas.scale;
            if let Some(control) = control_at(state.origin, x, y) {
                if let Some(text) = control.tooltip_string().and_then(|id| {
                    crate::win_tooltip::module_string(crate::win_tooltip::StringModule::Common, id)
                }) {
                    crate::win_tooltip::tool(ctx, egui::Id::new(("game-options", control)), &text);
                }
            }
        }
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            // The area itself must not claim the entire canvas; only actual controls do.
            if state.origin == GameOptionsOrigin::CommandCenter {
                // The name boxes (FUN_00604cf0: (119, y) 160 by 20) in the
                // default font; under a question they stay white and take
                // no input (A0 tooltip/211).
                let editable = !state.modal() && !state.suspended;
                for slot in 0..6 {
                    ui.put(
                        rect(119.0, 81.0 + slot as f32 * 42.0, 160.0, 20.0),
                        egui::TextEdit::singleline(&mut state.names[slot])
                            .id_source(("options_save_name", slot))
                            .frame(false)
                            .interactive(editable)
                            .margin(egui::Margin::ZERO)
                            .vertical_align(egui::Align::Center)
                            .font(crate::theme::original_font(10, canvas.scale))
                            .text_color(Color32::WHITE),
                    );
                }
            }
            // A question (FUN_00416c50: REBDLOG 10623, Yes and No) or the
            // message after a save (FUN_00417020: 10622, one button at
            // (176, 134) in the dialog).
            let dialog = match (&state.pending, state.saved_notice) {
                (Some(action), _) => Some((10623, confirmation_strings(action), false)),
                (None, Some(saved)) => Some((10622, (None, saved_notice_string(saved)), true)),
                (None, None) => None,
            };
            if let Some((background, (first, second), notice)) = dialog {
                let assets = [background, 10624, 10625, 10626, 10627];
                let complete = assets.iter().all(|id| {
                    cache.get(ctx, DllSource::Rebdlog, *id).is_some()
                        && cache
                            .original_resource_size(DllSource::Rebdlog, *id)
                            .is_some()
                });
                let bounds = rect(114.0, 152.0, 412.0, 176.0);
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                // The screen is 8-bit: GDI shows the dialog's DIBs in the
                // window's realized palette, COMMON 20002's with entry 254 white
                // (A0 tooltip/211, 212: every pixel of 10623, 10625 and 10627
                // is its nearest entry).
                let palette: Vec<[u8; 3]> = cache
                    .original_palette_rgba(DllSource::Common, resources::common::GAME_OPTIONS_BG)
                    .map(|entries| {
                        let mut palette: Vec<[u8; 3]> =
                            entries.iter().map(|[r, g, b, _]| [*r, *g, *b]).collect();
                        palette[254] = [255, 255, 255];
                        palette
                    })
                    .unwrap_or_default();
                let dialog_texture = |cache: &mut BmpCache, id: u32| {
                    if palette.is_empty() {
                        cache
                            .get(ctx, DllSource::Rebdlog, id)
                            .map(|texture| texture.id())
                    } else {
                        cache
                            .get_in_screen_palette(ctx, DllSource::Rebdlog, id, "options", &palette)
                            .map(|texture| texture.id())
                    }
                };
                if complete {
                    if let Some(texture) = dialog_texture(cache, background) {
                        ui.painter().image(texture, bounds, uv, Color32::WHITE);
                    }
                } else {
                    ui.painter()
                        .rect_filled(bounds, 4.0, Color32::from_rgb(30, 30, 35));
                }
                // Class_00658950_vt3c: the question's first string in font 5,
                // 0x2f0fbff, centred and word-wrapped from the top of (43, 25)
                // to (368, 86); the second centred on one line in (43, 87) to
                // (368, 112), or in (43, 25) to (368, 112) alone.
                let font = crate::theme::original_font(5, canvas.scale);
                let cream = Color32::from_rgb(255, 251, 240);
                let (left, top) = (114.0 + 43.0, 152.0);
                if let Some(first) = &first {
                    let mut job = egui::text::LayoutJob::simple(
                        first.clone(),
                        font.clone(),
                        cream,
                        325.0 * canvas.scale,
                    );
                    job.halign = egui::Align::Center;
                    let galley = ui.painter().layout_job(job);
                    let anchor = rect(left + 162.5, top + 25.0, 0.0, 0.0).min;
                    ui.painter().galley(anchor, galley, cream);
                }
                let (band_top, band_height) = if first.is_some() {
                    (87.0, 25.0)
                } else {
                    (25.0, 87.0)
                };
                let galley = ui.painter().layout_no_wrap(second, font, cream);
                // DT_VCENTER: (height - 16) / 2 whole pixels.
                let line_top = top + band_top + ((band_height - 16.0) / 2.0_f32).floor();
                let anchor = rect(left + 162.5, line_top, 0.0, 0.0).min
                    - Vec2::new(galley.size().x / 2.0, 0.0);
                ui.painter().galley(anchor, galley, cream);
                // FUN_00416e90: Yes shows 0x2981 and 0x2980 when held, No 0x2983
                // and 0x2982 (A0 tooltip/211, 212 show 10625 and 10627). The
                // message's one button (Class_006589a8_vt40) has Yes's art at
                // (290, 286).
                let buttons: &[(f32, f32, u32, u32, bool, &str)] = if notice {
                    &[(290.0, 286.0, 10625, 10624, true, "Yes")]
                } else {
                    &[
                        (252.0, 287.0, 10625, 10624, true, "Yes"),
                        (343.0, 287.0, 10627, 10626, false, "No"),
                    ]
                };
                // Class_00658950_vt4c / Class_006589a8_vt4c: Enter answers Yes.
                if ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
                    answer = Some(true);
                }
                for &(x, y, normal, pressed, value, label) in buttons {
                    let bounds = rect(x, y, 57.0, 28.0);
                    if complete {
                        let response =
                            ui.interact(bounds, ui.id().with(normal), egui::Sense::click());
                        // FUN_00416e90: REBDLOG 0x1300 "Yes", 0x1301 "No" (A0
                        // tooltip/211, 212).
                        if let Some(text) = crate::win_tooltip::module_string(
                            crate::win_tooltip::StringModule::Dialog,
                            if value { 0x1300 } else { 0x1301 },
                        ) {
                            if response.hovered() {
                                crate::win_tooltip::tool(ctx, response.id, &text);
                            }
                        }
                        let id = if response.is_pointer_button_down_on() {
                            pressed
                        } else {
                            normal
                        };
                        if let Some(texture) = dialog_texture(cache, id) {
                            ui.painter().image(texture, bounds, uv, Color32::WHITE);
                        }
                        let capture_id = ui.id().with((normal, "source_capture"));
                        if ctx.input(|i| i.pointer.primary_pressed()) {
                            let hit = ctx.pointer_latest_pos().is_some_and(|pos| {
                                bounds.contains(pos)
                                    && cache.is_resource_hit(
                                        DllSource::Rebdlog,
                                        normal,
                                        ((pos.x - bounds.left()) / canvas.scale) as usize,
                                        ((pos.y - bounds.top()) / canvas.scale) as usize,
                                    )
                            });
                            ui.data_mut(|data| data.insert_temp(capture_id, hit));
                        }
                        let captured =
                            ui.data(|data| data.get_temp::<bool>(capture_id).unwrap_or(false));
                        if captured
                            && response.clicked()
                            && response.interact_pointer_pos().is_some_and(|pos| {
                                cache.is_resource_hit(
                                    DllSource::Rebdlog,
                                    normal,
                                    ((pos.x - bounds.left()) / canvas.scale) as usize,
                                    ((pos.y - bounds.top()) / canvas.scale) as usize,
                                )
                            })
                        {
                            answer = Some(value);
                        }
                    } else {
                        let response =
                            ui.put(bounds, egui::Button::new(label).min_size(bounds.size()));
                        if response.clicked() {
                            answer = Some(value);
                        }
                    }
                }
            }
            if let Some(error) = &state.error {
                ui.painter().text(
                    rect(25.0, 440.0, 590.0, 20.0).left_center(),
                    Align2::LEFT_CENTER,
                    error,
                    FontId::monospace(11.0 * canvas.scale),
                    Color32::LIGHT_RED,
                );
            }
        });
    if state.saved_notice.is_some() {
        if answer.is_some() {
            state.saved_notice = None;
        }
        return GameOptionsAction::None;
    }
    answer.map_or(GameOptionsAction::None, |accepted| state.confirm(accepted))
}

/// The message after a save (`FUN_00408240`): REBDLOG 0x1217 or 0x1219.
fn saved_notice_string(saved: bool) -> String {
    let (id, shipped) = if saved {
        (0x1217, "Selected Game Saved Successfully")
    } else {
        (0x1219, "Selected Game Failed To Save")
    };
    crate::win_tooltip::module_string(crate::win_tooltip::StringModule::Dialog, id)
        .unwrap_or_else(|| shipped.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_control_has_its_tooltip_string() {
        // FUN_00407180 (TEXTCOMM; A0 tooltip/201..209).
        use GameOptionsControl as C;
        let tips = [
            C::Save(0),
            C::Load(5),
            C::MusicVolume,
            C::SoundVolume,
            C::Restart,
            C::Return,
            C::Exit,
            C::Music,
            C::Tactical(0),
        ]
        .map(C::tooltip_string);
        assert_eq!(
            tips,
            [
                Some(0x1010),
                Some(0x1011),
                Some(0x1012),
                Some(0x1013),
                Some(0x1014),
                Some(0x1015),
                Some(0x1018),
                None,
                None
            ]
        );
    }

    #[test]
    fn occupied_legacy_slots_wait_for_confirmation_and_cancellation_preserves_them() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        state.refresh_saves(&[SaveSlotInfo {
            slot: 9,
            name: "Legacy".into(),
            timestamp: String::new(),
            game_tick: 0,
            player_is_alliance: None,
        }]);
        let action = GameOptionsAction::Load { slot: 9 };
        assert_eq!(state.request(action.clone()), GameOptionsAction::None);
        assert_eq!(state.pending, Some(action.clone()));
        assert_eq!(state.confirm(false), GameOptionsAction::None);
        assert!(state.occupied[9]);
        assert_eq!(state.request(action.clone()), GameOptionsAction::None);
        assert_eq!(state.confirm(true), action);
        assert_eq!(state.confirm(true), GameOptionsAction::None);
    }

    #[test]
    fn saves_require_valid_names_and_slots_and_overwrites_require_confirmation() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        for slot in [0, 9] {
            let save = GameOptionsAction::Save {
                slot,
                name: "Campaign".into(),
            };
            assert_eq!(state.request(save.clone()), save);
            state.occupied[slot] = true;
            assert_eq!(state.request(save.clone()), GameOptionsAction::None);
            assert_eq!(state.pending, Some(save.clone()));
            assert_eq!(state.confirm(false), GameOptionsAction::None);
            state.request(save.clone());
            assert_eq!(state.confirm(true), save);
        }
        // FUN_00406b80 / FUN_00408000 do not check the name: a blank one
        // saves.
        let blank = GameOptionsAction::Save {
            slot: 1,
            name: " ".into(),
        };
        assert_eq!(state.request(blank.clone()), blank);
        for (slot, name) in [(10, "Name"), (usize::MAX, "Name")] {
            assert_eq!(
                state.request(GameOptionsAction::Save {
                    slot,
                    name: name.into()
                }),
                GameOptionsAction::None
            );
            assert!(state.pending.is_none());
        }
        state.suspended = true;
        assert_eq!(
            state.request(GameOptionsAction::Exit),
            GameOptionsAction::None
        );
        assert!(state.pending.is_none());
        state.suspended = false;
        state.request(GameOptionsAction::Exit);
        assert_eq!(
            state.request(GameOptionsAction::Return),
            GameOptionsAction::None
        );
        assert_eq!(state.pending, Some(GameOptionsAction::Exit));
        assert_eq!(state.escape(), GameOptionsAction::None);
        assert_eq!(state.escape(), GameOptionsAction::Return);
    }

    #[test]
    fn tactical_options_reject_save_load_delete_and_restart() {
        // FUN_00407180: states 20–23 disable campaign save and load controls.
        let mut state = GameOptionsState::new(GameOptionsOrigin::TacticalBattle);
        state.occupied[0] = true;
        for action in [
            GameOptionsAction::Save {
                slot: 0,
                name: "Battle".into(),
            },
            GameOptionsAction::Load { slot: 0 },
            GameOptionsAction::Delete { slot: 0 },
            GameOptionsAction::Restart,
        ] {
            assert_eq!(state.request(action), GameOptionsAction::None);
            assert!(state.pending.is_none());
        }
        assert!(!GameOptionsOrigin::TacticalBattle.can_restart());
        // A0 main-menu-states/12: Restart is on from the shuttle cockpit.
        assert!(GameOptionsOrigin::ShuttleCockpit.can_restart());
        assert!(GameOptionsOrigin::CommandCenter.can_restart());
    }

    #[test]
    fn missing_dialog_assets_still_allow_confirmation_and_cancellation() {
        use egui_macroquad::egui::{self, Pos2, Rect, Vec2};
        for accepted in [false, true] {
            let ctx = egui::Context::default();
            crate::theme::load_fonts(&ctx);
            let mut cache = BmpCache::new();
            let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
            state.request(GameOptionsAction::Exit);
            let pos = Pos2::new(if accepted { 280.0 } else { 371.0 }, 301.0);
            let mut emitted = GameOptionsAction::None;
            for down in [None, Some(true), Some(false)] {
                let mut events = vec![egui::Event::PointerMoved(pos)];
                if let Some(pressed) = down {
                    events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    });
                }
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        let action = draw_game_options_overlay(ctx, &mut cache, &mut state);
                        if action != GameOptionsAction::None {
                            emitted = action;
                        }
                    },
                );
            }
            assert!(state.pending.is_none(), "fallback must consume the answer");
            assert_eq!(
                emitted,
                if accepted {
                    GameOptionsAction::Exit
                } else {
                    GameOptionsAction::None
                }
            );
        }
    }

    #[test]
    fn bitmap_controls_reject_transparent_corners_and_exact_outer_edges() {
        // FUN_005fca00: palette-key hit mask and strict top/left/outer edges.
        let root = std::env::temp_dir().join(format!("options-hit-mask-{}", std::process::id()));
        let dir = root.join("common-dll/BMP");
        std::fs::create_dir_all(&dir).unwrap();
        let (width, height, stride, offset) = (19usize, 35usize, 20usize, 1078usize);
        let mut bmp = vec![0u8; offset + stride * height];
        bmp[..2].copy_from_slice(b"BM");
        bmp[10..14].copy_from_slice(&(offset as u32).to_le_bytes());
        bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&(width as i32).to_le_bytes());
        bmp[22..26].copy_from_slice(&(height as i32).to_le_bytes());
        bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
        bmp[28..30].copy_from_slice(&8u16.to_le_bytes());
        bmp[offset..].fill(3);
        bmp[offset] = 7;
        bmp[offset + (height - 2) * stride + 1] = 7;
        std::fs::write(dir.join("10040.bmp"), bmp).unwrap();
        let mut cache = BmpCache::new();
        cache.set_base_path(&root);
        let origin = GameOptionsOrigin::CommandCenter;
        assert_eq!(
            masked_control_at(&mut cache, origin, 356.0, 80.0),
            Some(GameOptionsControl::Music)
        );
        for (x, y) in [
            (352.0, 80.0),
            (356.0, 76.0),
            (371.0, 80.0),
            (356.0, 111.0),
            (353.0, 77.0),
        ] {
            assert_eq!(masked_control_at(&mut cache, origin, x, y), None, "{x},{y}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hit_regions_reject_exact_outer_edges() {
        // No recovered source: hit-region pixel coordinates, kept as a regression pin.
        // FUN_00407180's literal rectangles: Play Music (352, 76) 19 by 35,
        // Save (34, 81 + 42k), Load (287, 81 + 42k).
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 352.0, 76.0),
            Some(GameOptionsControl::Music)
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 371.0, 76.0),
            None
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 34.0, 81.0),
            Some(GameOptionsControl::Save(0))
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 287.0, 291.0),
            Some(GameOptionsControl::Load(5))
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 34.0, 165.0),
            Some(GameOptionsControl::Save(2))
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 75.0, 310.0),
            Some(GameOptionsControl::Save(5))
        );
    }

    #[test]
    fn shuttle_and_tactical_contexts_disable_original_controls() {
        // A0 main-menu-states/12: from the shuttle cockpit Restart (76,
        // 381) is on and Return (162, 382) off.
        assert_eq!(
            control_at(GameOptionsOrigin::ShuttleCockpit, 80.0, 400.0),
            Some(GameOptionsControl::Restart)
        );
        assert_eq!(
            control_at(GameOptionsOrigin::ShuttleCockpit, 175.0, 400.0),
            None
        );
        assert_eq!(
            control_at(GameOptionsOrigin::TacticalBattle, 370.0, 320.0),
            None
        );
        assert_eq!(
            control_at(GameOptionsOrigin::TacticalBattle, 175.0, 400.0),
            Some(GameOptionsControl::Return)
        );
    }

    #[test]
    fn tactical_flags_default_to_on() {
        // No recovered source: default tactical-option on state, kept as a regression pin.
        assert_eq!(GameOptionsState::default().tactical_flags(), [true; 5]);
    }

    #[test]
    fn a_slider_thumb_centres_on_its_whole_percent() {
        // FUN_00604570: thumb centre floor(201 * v / 100).
        assert_eq!(slider_thumb_centre(0.0), 0.0);
        assert_eq!(slider_thumb_centre(0.5), 100.0);
        assert_eq!(slider_thumb_centre(0.75), 150.0);
        assert_eq!(slider_thumb_centre(1.0), 201.0);
        assert_eq!(slider_thumb_centre(2.0), 201.0);
    }

    #[test]
    fn a_slider_drag_takes_only_points_inside_its_five_pixel_margins() {
        // FUN_00604950: x strictly within 5..196, value floor(x * 100 / 201).
        let mut audio = AudioVolumeState {
            music_volume: 0.4,
            dirty: false,
            ..Default::default()
        };
        for x in [392.0 + 5.0, 392.0 + 196.0, 300.0] {
            set_slider(&mut audio, GameOptionsControl::MusicVolume, x);
            assert_eq!(audio.music_volume, 0.4, "{x}");
            assert!(!audio.dirty);
        }
        set_slider(&mut audio, GameOptionsControl::MusicVolume, 392.0 + 6.0);
        assert_eq!(audio.music_volume, 0.02);
        assert!(audio.dirty);
        set_slider(&mut audio, GameOptionsControl::SoundVolume, 392.0 + 195.9);
        assert_eq!(audio.sfx_volume, 0.97);
        set_slider(&mut audio, GameOptionsControl::SoundVolume, 392.0 + 100.0);
        assert_eq!(audio.sfx_volume, 0.49);
    }

    #[test]
    fn the_return_button_art_follows_the_window_mode_and_side() {
        // FUN_00407180 switch on +0x9c; A0 200-options (Empire, galaxy)
        // shows 10023.
        assert_eq!(
            return_art(GameOptionsOrigin::CommandCenter, false),
            [10023, 10024, 10025]
        );
        assert_eq!(
            return_art(GameOptionsOrigin::CommandCenter, true),
            [10020, 10021, 10022]
        );
        assert_eq!(
            return_art(GameOptionsOrigin::TacticalBattle, false),
            [10026, 10027, 10028]
        );
        assert_eq!(
            return_art(GameOptionsOrigin::TacticalBattle, true),
            [10026, 10027, 10028]
        );
        assert_eq!(
            return_art(GameOptionsOrigin::ShuttleCockpit, false),
            [10020, 10021, 10022]
        );
    }

    #[test]
    fn an_empty_slot_has_an_empty_name() {
        // FUN_00407180: an empty slot's name box is empty.
        let mut state = GameOptionsState::default();
        assert!(state.names.iter().all(String::is_empty));
        state.refresh_saves(&[SaveSlotInfo {
            slot: 2,
            name: "Campaign".into(),
            timestamp: String::new(),
            game_tick: 0,
            player_is_alliance: Some(false),
        }]);
        assert_eq!(state.names[2], "Campaign");
        assert!(state.names[1].is_empty() && state.names[3].is_empty());
    }

    #[test]
    fn the_saved_message_holds_the_window_until_it_is_answered() {
        // FUN_00408240 -> FUN_00417020; Class_006589a8_vt4c closes it on
        // Enter only.
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        state.saved_notice = Some(true);
        assert!(state.modal());
        assert_eq!(state.escape(), GameOptionsAction::None);
        assert_eq!(state.saved_notice, Some(true));
        assert_eq!(
            state.request(GameOptionsAction::Exit),
            GameOptionsAction::None
        );
        assert!(state.pending.is_none());
        state.set_origin(GameOptionsOrigin::CommandCenter);
        assert!(!state.modal());
        assert_eq!(
            saved_notice_string(true),
            "Selected Game Saved Successfully"
        );
        assert_eq!(saved_notice_string(false), "Selected Game Failed To Save");
    }

    #[test]
    fn each_question_asks_the_originals_strings() {
        // FUN_00408170 (0x476..0x478) and the quit question 0x1202.
        let line =
            |first: Option<&str>, second: &str| (first.map(str::to_owned), second.to_owned());
        assert_eq!(
            confirmation_strings(&GameOptionsAction::Restart),
            line(
                Some("Returning to the shuttle cockpit will cause unsaved changes to be lost"),
                "Return without saving?"
            )
        );
        assert_eq!(
            confirmation_strings(&GameOptionsAction::Load { slot: 0 }),
            line(
                Some("Loading the selected game will destroy unsaved changes"),
                "Load without saving?"
            )
        );
        assert_eq!(
            confirmation_strings(&GameOptionsAction::Save {
                slot: 0,
                name: String::new()
            }),
            line(
                Some("Saving the selected game will destroy a previously saved game"),
                "Save anyway?"
            )
        );
        assert_eq!(
            confirmation_strings(&GameOptionsAction::Exit),
            line(None, "Are you sure you want to quit?")
        );
        assert_eq!(
            confirmation_strings(&GameOptionsAction::Delete { slot: 0 }),
            line(None, "Delete the selected saved game?")
        );
    }

    #[test]
    fn restart_asks_only_in_a_game_with_nothing_saved_this_visit() {
        // FUN_00406b80 command 0x91 -> FUN_00408170 0x476.
        let mut shuttle = GameOptionsState::new(GameOptionsOrigin::ShuttleCockpit);
        assert_eq!(
            shuttle.request(GameOptionsAction::Restart),
            GameOptionsAction::Restart
        );
        let mut game = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        assert_eq!(
            game.request(GameOptionsAction::Restart),
            GameOptionsAction::None
        );
        assert_eq!(game.pending, Some(GameOptionsAction::Restart));
        game.confirm(false);
        game.saved_this_visit = true;
        assert_eq!(
            game.request(GameOptionsAction::Restart),
            GameOptionsAction::Restart
        );
        game.set_origin(GameOptionsOrigin::CommandCenter);
        assert!(!game.saved_this_visit);
    }
}
