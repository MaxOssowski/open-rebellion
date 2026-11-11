//! Original bitmap-driven unified Game Options surface.

use macroquad::prelude::*;

use crate::audio::AudioVolumeState;
use crate::bmp_cache::{resources, BmpCache, DllSource};

/// The window's save rows (`FUN_00407180` builds six: Save 0x6f..0x74, Load
/// 0x79..0x7e; manual p. 79 "There are six slots for saved games"). Slots
/// the data layer holds beyond them are neither listed nor loaded.
pub const SAVE_ROWS: usize = 6;

/// Metadata about one save slot, passed from the caller (who holds
/// `rebellion_data::save::SaveMeta`) down to the window, so that
/// `rebellion-render` stays free of the data layer.
#[derive(Debug, Clone)]
pub struct SaveSlotInfo {
    /// Slot index (`0..SAVE_ROWS`).
    pub slot: usize,
    /// Human-readable save name.
    pub name: String,
    /// Formatted timestamp string (e.g. "2026-03-15 14:32").
    pub timestamp: String,
    /// Game tick at save time.
    pub game_tick: u64,
    /// The player's side, when known (the row's crest).
    pub player_is_alliance: Option<bool>,
}

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
    pub occupied: [bool; SAVE_ROWS],
    pub names: [String; SAVE_ROWS],
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
    /// Each slider's thumb centre (music, effects) once the pointer has
    /// moved it in this visit (`FUN_00604950` writes the pointer to
    /// `+0xc8`); `None` keeps where the window opened it.
    slider_thumbs: [Option<f32>; 2],
}

impl Default for GameOptionsState {
    fn default() -> Self {
        Self {
            pending: None,
            saved_notice: None,
            saved_this_visit: false,
            occupied: [false; SAVE_ROWS],
            names: std::array::from_fn(|_| String::new()),
            error: None,
            origin: GameOptionsOrigin::ShuttleCockpit,
            pressed: None,
            show_starfield: true,
            show_planet: true,
            show_pyrotechnics: true,
            high_detail_models: true,
            display_holocube: true,
            player_is_alliance: true,
            slider_thumbs: [None; 2],
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
            // A0 TAC-06-C007: a battle's Game Options shows each occupied
            // slot's Load lit (10049) beside its grey Save (10048).
            GameOptionsAction::Load { slot } => self.occupied.get(*slot).copied().unwrap_or(false),
            // FUN_00407180 never disables Restart: from the shuttle cockpit
            // it shows 10035 and returns there at once (A0
            // main-menu-states/12), and a battle shows it lit (A0
            // TAC-06-C007).
            GameOptionsAction::Restart => true,
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
        if self.modal() || !self.enabled(&action) {
            return GameOptionsAction::None;
        }
        let confirm = match &action {
            GameOptionsAction::Save { slot, .. } => self.occupied[*slot],
            GameOptionsAction::Load { .. } => self.origin.has_live_campaign(),
            // FUN_00406b80 command 0x91: a game in progress (+0xa8) with
            // no save made in this visit asks 0x476 (FUN_00408170); else
            // Restart goes straight to the shuttle cockpit.
            GameOptionsAction::Restart => self.origin.has_live_campaign() && !self.saved_this_visit,
            GameOptionsAction::Exit => true,
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
        self.error = None;
        // FUN_00407180 builds the sliders anew at each opening.
        self.slider_thumbs = [None; 2];
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
        GameOptionsAction::None | GameOptionsAction::Return => (None, String::new()),
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
        if load_rect(slot).contains(x, y) {
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
    if RESTART_RECT.contains(x, y) {
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
/// The display options during a battle (`FUN_00408bc0`, states 20..23):
/// `COLORREF` 0x2505050, a PALETTERGB, which the text takes as the default
/// palette's nearest colour, the system grey (128, 128, 128). A0
/// TAC-06-C007 draws them so; the window's other PALETTERGBs, 0x200ff00 and
/// 0x2008000, are system colours themselves.
const OPTION_DISABLED: Color = Color::new(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0);

/// A TEXTCOMM string, as shipped when the table is not loaded.
fn textcomm(id: u16, shipped: &str) -> String {
    crate::win_tooltip::module_string(crate::win_tooltip::StringModule::Common, id)
        .unwrap_or_else(|| shipped.to_owned())
}

/// A volume slider (`FUN_00604570`, painted by `FUN_00604a50`): the whole
/// track at the slider's origin, then the thumb (COMMON 10054, 11 by 47)
/// centred on `centre` at y + 4, clipped to the track's 201 by 54. A0
/// 200-options shows the track's palette entry 254 (cyan in the DIB) as
/// white on this screen.
fn draw_volume(cache: &mut BmpCache, canvas: OptionsCanvas, y: f32, centre: f32, alternate: bool) {
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
    let (left, visible) = thumb_span(centre, thumb.width());
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

/// The thumb's left in slider pixels and its width left visible once the
/// track's 201 pixels clip it (`FUN_00604a50`), for a thumb `width` wide
/// centred on `centre`.
fn thumb_span(centre: f32, width: f32) -> (f32, f32) {
    let left = centre - (width / 2.0).floor();
    (left, (SLIDER_WIDTH - left).min(width).max(0.0))
}

/// The slider's left edge (`FUN_00407180`: x 0x188) and client width.
const SLIDER_X: f32 = 392.0;
const SLIDER_WIDTH: f32 = 201.0;
const SLIDER_HEIGHT: f32 = 54.0;

/// The volume a slider at 0 sends (`FUN_00406b80` commands 0x8d/0x8e:
/// `mov eax, 0xffffda80`), and below which `FUN_00407180` shows 0.
const SILENT_HUNDREDTHS: i32 = -9600;

/// A slider percent as the DirectSound volume in hundredths of a decibel
/// (`FUN_00406b80` commands 0x8d/0x8e at 0x406de5..0x406e0d:
/// `floor(log10(v * 0.01) * 2000)`, -9600 for 0).
/// hyp: computed in f64, the precision of the MSVC runtime's default x87
/// control word (53-bit); the original's precision is not observed.
fn slider_hundredths(percent: u32) -> i32 {
    if percent == 0 {
        return SILENT_HUNDREDTHS;
    }
    ((f64::from(percent) * 0.01).log10() * 2000.0).floor() as i32
}

/// The percent a slider opens at for a DirectSound volume (`FUN_00407180`
/// at 0x40773b..0x407780: `floor(10^(vol * 0.01 * 0.05) * 100)`, 0 below
/// -9600), so a sent percent other than 1, 10 and 100 reopens one lower.
/// hyp: f64, as for [`slider_hundredths`].
fn slider_percent(hundredths: i32) -> u32 {
    if hundredths < SILENT_HUNDREDTHS {
        return 0;
    }
    (10f64.powf(f64::from(hundredths) * 0.01 * 0.05) * 100.0).floor() as u32
}

/// DirectSound's amplitude for a volume in hundredths of a decibel of
/// attenuation (`IDirectSoundBuffer::SetVolume`): `10^(vol / 2000)`. The
/// port's mixer takes the amplitude.
fn hundredths_gain(hundredths: i32) -> f32 {
    10f64.powf(f64::from(hundredths) / 2000.0) as f32
}

/// The DirectSound volume an amplitude stands for: the inverse of
/// [`hundredths_gain`], exact for the amplitudes it gives.
/// port: any other amplitude (an older linear setting, the status-bar
/// slider) takes the nearest hundredth; 0 is DirectSound's minimum, -10000.
fn gain_hundredths(gain: f32) -> i32 {
    if gain.is_nan() || gain <= 0.0 {
        return -10_000;
    }
    (2000.0 * f64::from(gain).log10())
        .round()
        .clamp(-10_000.0, 0.0) as i32
}

/// The thumb's centre in slider pixels as the window opens for a volume
/// amplitude (`FUN_00604570`: `floor(201 * v / 100)` of the percent
/// `FUN_00407180` derives).
fn slider_thumb_centre(gain: f32) -> f32 {
    (SLIDER_WIDTH * slider_percent(gain_hundredths(gain)) as f32 / 100.0).floor()
}

/// The percent a thumb centre stands for (`FUN_00604a50`:
/// `+0xdc = cx * 100 / 201`, recomputed at each paint).
fn thumb_percent(centre: f32) -> u32 {
    (centre.max(0.0) * 100.0 / SLIDER_WIDTH).floor() as u32
}

/// Whether a point is strictly inside a slider (`FUN_006048f0`); only then
/// does a press move the thumb or a move send the value.
fn inside_slider(control: GameOptionsControl, x: f32, y: f32) -> bool {
    let Some(index) = slider_index(control) else {
        return false;
    };
    let rect = slider_rect(index);
    let (x, y) = ((x - rect.x).floor(), (y - rect.y).floor());
    x > 0.0 && x < SLIDER_WIDTH && y > 0.0 && y < SLIDER_HEIGHT
}

const fn slider_index(control: GameOptionsControl) -> Option<usize> {
    match control {
        GameOptionsControl::MusicVolume => Some(0),
        GameOptionsControl::SoundVolume => Some(1),
        _ => None,
    }
}

const fn slider_rect(index: usize) -> NativeRect {
    if index == 0 {
        MUSIC_SLIDER_RECT
    } else {
        SOUND_SLIDER_RECT
    }
}

/// The control a press goes to: a slider takes it anywhere in its window
/// (no key-colour mask, `FUN_006048f0`), any other control only on its
/// opaque pixels (`masked`).
fn press_target(
    at: Option<GameOptionsControl>,
    masked: impl FnOnce() -> Option<GameOptionsControl>,
) -> Option<GameOptionsControl> {
    match at {
        Some(control) if slider_index(control).is_some() => Some(control),
        _ => masked(),
    }
}

impl GameOptionsState {
    /// `Class_0066e010_vt14`: a move with the button held and the pointer
    /// strictly inside the pressed slider moves the thumb and sends the
    /// value.
    fn held_slider_move(&mut self, audio: &mut AudioVolumeState, pointer: (f32, f32), down: bool) {
        if let Some(control) = self.pressed_slider() {
            if down && inside_slider(control, pointer.0, pointer.1) {
                self.move_thumb(control, pointer.0, pointer.1);
                self.send_slider(audio, control);
            }
        }
    }

    /// The slider the button went down on, if any.
    fn pressed_slider(&self) -> Option<GameOptionsControl> {
        self.pressed
            .filter(|control| slider_index(*control).is_some())
    }

    /// The thumb's centre: where the pointer put it in this visit, else
    /// where the window opened it.
    fn thumb_centre(&self, audio: &AudioVolumeState, index: usize) -> f32 {
        self.slider_thumbs[index].unwrap_or_else(|| {
            slider_thumb_centre(if index == 0 {
                audio.music_volume
            } else {
                audio.sfx_volume
            })
        })
    }

    /// Move a slider's thumb to the pointer (`FUN_00604950`): the thumb
    /// centres on the pointer only strictly between x 5 and 196 and y 0 and
    /// 54 in the slider, so a drag gives 2..97 per cent, never 0 or 100.
    fn move_thumb(&mut self, control: GameOptionsControl, logical_x: f32, logical_y: f32) {
        let Some(index) = slider_index(control) else {
            return;
        };
        let rect = slider_rect(index);
        let (x, y) = ((logical_x - rect.x).floor(), (logical_y - rect.y).floor());
        if x > 5.0 && x < SLIDER_WIDTH - 5.0 && y > 0.0 && y < SLIDER_HEIGHT {
            self.slider_thumbs[index] = Some(x);
        }
    }

    /// Send a slider's value (`Class_0066e010_vt14` posts `+0xdc` on a move
    /// with the button held and on the release; `FUN_00406b80` turns it
    /// into the DirectSound volume, which `FUN_00417670` / `FUN_00416360`
    /// apply and keep in the registry).
    fn send_slider(&self, audio: &mut AudioVolumeState, control: GameOptionsControl) {
        let Some(index) = slider_index(control) else {
            return;
        };
        let percent = thumb_percent(self.thumb_centre(audio, index));
        let gain = hundredths_gain(slider_hundredths(percent));
        let current = if index == 0 {
            &mut audio.music_volume
        } else {
            &mut audio.sfx_volume
        };
        if current.to_bits() != gain.to_bits() {
            *current = gain;
            audio.dirty = true;
        }
    }
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
        state.thumb_centre(audio, 0),
        false,
    );
    draw_volume(
        cache,
        canvas,
        SOUND_SLIDER_RECT.y,
        state.thumb_centre(audio, 1),
        true,
    );

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
        let load_resource = if occupied.is_none() {
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
        if held == Some(GameOptionsControl::Restart) {
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

    if state.modal() {
        state.pressed = None;
        return GameOptionsAction::None;
    }
    // Class_0066e010_vt14: a move with the button held and the pointer
    // inside moves the thumb and sends the value.
    state.held_slider_move(audio, pointer, is_mouse_button_down(MouseButton::Left));
    if is_mouse_button_pressed(MouseButton::Left) {
        // A slider takes a press anywhere in its window (no key-colour
        // mask), captures the mouse and moves the thumb; it sends nothing
        // until the pointer moves or the button is released.
        state.pressed = press_target(control_at(state.origin, pointer.0, pointer.1), || {
            masked_control_at(cache, state.origin, pointer.0, pointer.1)
        });
        if let Some(control) = state.pressed_slider() {
            if inside_slider(control, pointer.0, pointer.1) {
                state.move_thumb(control, pointer.0, pointer.1);
            }
        }
    }
    if is_mouse_button_released(MouseButton::Left) {
        // The capture's release sends the slider's value wherever the
        // pointer is.
        if let Some(control) = state.pressed_slider() {
            state.send_slider(audio, control);
        }
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
    if !state.modal() {
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
                let editable = !state.modal();
                // The default font, as the shown name (A0 200-options: "test"
                // at 13 ppem). The EDIT control sets its line 1 below the
                // box's top (A0: baseline at y + 14, tmAscent 13); the box
                // moves up by egui's own baseline in the line.
                let font = crate::theme::gdi_font(crate::DEFAULT_FONT, canvas.scale);
                let egui_baseline = ui
                    .painter()
                    .layout_no_wrap("A".to_owned(), font.clone(), Color32::WHITE)
                    .rows
                    .first()
                    .and_then(|row| row.glyphs.first())
                    .map_or(0.0, |glyph| glyph.pos.y);
                for slot in 0..6 {
                    let (x, y, w, h) = (119.0, 81.0 + slot as f32 * 42.0, 160.0, 20.0);
                    let (_, baseline) = crate::theme::gdi_text_origin(
                        (x, y + 1.0, w, h),
                        crate::DEFAULT_FONT,
                        0.0,
                        (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                    );
                    let mut box_rect = rect(x, y, w, h);
                    box_rect = box_rect.translate(egui::vec2(
                        0.0,
                        rect(x, baseline, 0.0, 0.0).min.y - egui_baseline - box_rect.min.y,
                    ));
                    ui.put(
                        box_rect,
                        egui::TextEdit::singleline(&mut state.names[slot])
                            .id_source(("options_save_name", slot))
                            .frame(false)
                            .interactive(editable)
                            .margin(egui::Margin::ZERO)
                            .vertical_align(egui::Align::Min)
                            .font(font.clone())
                            .text_color(Color32::WHITE),
                    );
                }
            }
            // A question (FUN_00416c50: REBDLOG 10623, Yes and No) or the
            // message after a save (FUN_00417020: 10622, one button at
            // (176, 134) in the dialog).
            let dialog = match (&state.pending, state.saved_notice) {
                (Some(action), _) => Some((
                    crate::rebdlog_dialog::Kind::Question,
                    confirmation_strings(action),
                )),
                (None, Some(saved)) => Some((
                    crate::rebdlog_dialog::Kind::Message,
                    (None, saved_notice_string(saved)),
                )),
                (None, None) => None,
            };
            if let Some((kind, (first, second))) = dialog {
                // GDI shows the dialog in the window's realized palette,
                // COMMON 20002's with entry 254 white (A0 tooltip/211, 212).
                let palette = crate::rebdlog_dialog::realized_palette(
                    cache,
                    DllSource::Common,
                    resources::common::GAME_OPTIONS_BG,
                );
                answer = crate::rebdlog_dialog::paint(
                    ui,
                    ctx,
                    cache,
                    crate::rebdlog_dialog::Screen {
                        origin: Pos2::new(canvas.offset_x, canvas.offset_y),
                        scale: canvas.scale,
                    },
                    kind,
                    &crate::rebdlog_dialog::Texts {
                        first: first.as_deref(),
                        second: &second,
                    },
                    &palette,
                    "options",
                );
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
    fn occupied_slots_wait_for_confirmation_and_cancellation_preserves_them() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        state.refresh_saves(&[SaveSlotInfo {
            slot: 5,
            name: "Legacy".into(),
            timestamp: String::new(),
            game_tick: 0,
            player_is_alliance: None,
        }]);
        let action = GameOptionsAction::Load { slot: 5 };
        assert_eq!(state.request(action.clone()), GameOptionsAction::None);
        assert_eq!(state.pending, Some(action.clone()));
        assert_eq!(state.confirm(false), GameOptionsAction::None);
        assert!(state.occupied[5]);
        assert_eq!(state.request(action.clone()), GameOptionsAction::None);
        assert_eq!(state.confirm(true), action);
        assert_eq!(state.confirm(true), GameOptionsAction::None);
    }

    #[test]
    fn a_save_beyond_the_six_rows_is_neither_listed_nor_loadable() {
        // FUN_00407180 builds six rows (Load 0x79..0x7e); manual p. 79:
        // "There are six slots for saved games".
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        let save = |slot| SaveSlotInfo {
            slot,
            name: format!("Slot {slot}"),
            timestamp: String::new(),
            game_tick: 0,
            player_is_alliance: None,
        };
        state.refresh_saves(&[save(2), save(7), save(9)]);
        let listed: Vec<usize> = (0..state.occupied.len())
            .filter(|&slot| state.occupied[slot])
            .collect();
        assert_eq!(listed, [2]);
        assert_eq!(state.names[2], "Slot 2");
        for slot in [7, 9] {
            assert_eq!(
                state.request(GameOptionsAction::Load { slot }),
                GameOptionsAction::None
            );
            assert!(state.pending.is_none());
        }
    }

    #[test]
    fn saves_require_valid_names_and_slots_and_overwrites_require_confirmation() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        for slot in [0, 5] {
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
        for (slot, name) in [(6, "Name"), (10, "Name"), (usize::MAX, "Name")] {
            assert_eq!(
                state.request(GameOptionsAction::Save {
                    slot,
                    name: name.into()
                }),
                GameOptionsAction::None
            );
            assert!(state.pending.is_none());
        }
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
    fn tactical_options_reject_save_but_ask_load_and_restart() {
        // A0 TAC-06-C007 (a battle's Game Options): every Save grey (10048),
        // each occupied slot's Load lit (10049).
        let mut state = GameOptionsState::new(GameOptionsOrigin::TacticalBattle);
        state.occupied[0] = true;
        for action in [
            GameOptionsAction::Save {
                slot: 0,
                name: "Battle".into(),
            },
            GameOptionsAction::Load { slot: 1 },
        ] {
            assert_eq!(state.request(action), GameOptionsAction::None);
            assert!(state.pending.is_none());
        }
        // An occupied slot's Load asks first, the game being in progress.
        assert_eq!(
            state.request(GameOptionsAction::Load { slot: 0 }),
            GameOptionsAction::None
        );
        assert_eq!(state.pending, Some(GameOptionsAction::Load { slot: 0 }));
        state.confirm(false);
        assert_eq!(
            control_at(GameOptionsOrigin::TacticalBattle, 300.0, 90.0),
            Some(GameOptionsControl::Load(0))
        );
        // A0 TAC-06-C007: Restart is lit in a battle; with the game in
        // progress and nothing saved it asks 0x476 first (FUN_00406b80).
        assert_eq!(
            state.request(GameOptionsAction::Restart),
            GameOptionsAction::None
        );
        assert_eq!(state.pending, Some(GameOptionsAction::Restart));
        assert_eq!(
            control_at(GameOptionsOrigin::TacticalBattle, 80.0, 400.0),
            Some(GameOptionsControl::Restart)
        );
        // The display options' grey (A0 TAC-06-C007).
        assert_eq!(OPTION_DISABLED, Color::from_rgba(128, 128, 128, 255));
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
    fn a_slider_percent_becomes_floor_2000_log10_hundredths_of_a_decibel() {
        // FUN_00406b80 commands 0x8d/0x8e (0x406de5..0x406e0d): 0 sends
        // -9600 (0xffffda80), else floor(log10(v * 0.01) * 2000).
        assert_eq!(
            [0, 2, 47, 49, 50, 75, 97, 100].map(slider_hundredths),
            [-9600, -3398, -656, -620, -603, -250, -27, 0]
        );
    }

    #[test]
    fn a_slider_opens_at_floor_100_times_ten_to_the_volume_over_2000() {
        // FUN_00407180 (0x40773b..0x407780): below -9600 shows 0, else
        // floor(10^(vol * 0.01 * 0.05) * 100).
        assert_eq!(
            [-10_000, -9601, -9600, -620, -603, 0].map(slider_percent),
            [0, 0, 0, 48, 49, 100]
        );
    }

    #[test]
    fn a_sent_percent_reopens_one_lower_except_exact_powers_of_ten() {
        // FUN_00406b80 floors the decibels and FUN_00407180 floors the
        // percent, so only 1, 10 and 100 survive the round trip (in f64).
        let kept: Vec<u32> = (1..=100)
            .filter(|&v| slider_percent(slider_hundredths(v)) == v)
            .collect();
        assert_eq!(kept, [1, 10, 100]);
        for v in (1..=100).filter(|v| !kept.contains(v)) {
            assert_eq!(slider_percent(slider_hundredths(v)), v - 1, "{v}");
        }
    }

    #[test]
    fn the_mixer_gain_is_directsound_s_amplitude_for_the_volume() {
        // IDirectSoundBuffer::SetVolume: hundredths of a decibel of
        // attenuation, amplitude 10^(vol / 2000).
        assert_eq!(hundredths_gain(0), 1.0);
        assert!((hundredths_gain(-2000) - 0.1).abs() < 1e-7);
        assert!((hundredths_gain(-603) - 0.499_4).abs() < 1e-4);
        for hundredths in -10_000..=0 {
            assert_eq!(gain_hundredths(hundredths_gain(hundredths)), hundredths);
        }
        assert_eq!(gain_hundredths(0.0), -10_000);
        assert_eq!(gain_hundredths(2.0), 0);
    }

    #[test]
    fn a_slider_opens_its_thumb_at_the_percent_its_volume_reads_back_as() {
        // FUN_00604570: thumb centre floor(201 * v / 100) of FUN_00407180's
        // percent; a sent 50 (-603) reads back as 49, centre 98.
        assert_eq!(slider_thumb_centre(0.0), 0.0);
        assert_eq!(slider_thumb_centre(hundredths_gain(-603)), 98.0);
        assert_eq!(slider_thumb_centre(hundredths_gain(-250)), 148.0);
        assert_eq!(slider_thumb_centre(1.0), 201.0);
        assert_eq!(slider_thumb_centre(2.0), 201.0);
    }

    #[test]
    fn a_slider_drag_takes_only_points_inside_its_five_pixel_margins() {
        // FUN_00604950: the thumb centres on a pointer strictly within x
        // 5..196 and y 0..54; Class_0066e010_vt14 then sends
        // floor(x * 100 / 201).
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        let mut audio = AudioVolumeState::default();
        let music = GameOptionsControl::MusicVolume;
        for (x, y) in [(5.0, 20.0), (196.0, 20.0), (100.0, 0.0), (100.0, 54.0)] {
            state.move_thumb(music, 392.0 + x, 130.0 + y);
            assert_eq!(state.thumb_centre(&audio, 0), 201.0, "{x},{y}");
        }
        state.move_thumb(music, 392.0 + 6.0, 130.0 + 1.0);
        assert_eq!(state.thumb_centre(&audio, 0), 6.0);
        state.send_slider(&mut audio, music);
        assert_eq!(audio.music_volume, hundredths_gain(-3398));
        assert!(audio.dirty);

        let effects = GameOptionsControl::SoundVolume;
        state.move_thumb(effects, 392.0 + 195.9, 190.0 + 53.0);
        state.send_slider(&mut audio, effects);
        assert_eq!(audio.sfx_volume, hundredths_gain(slider_hundredths(97)));
        state.move_thumb(effects, 392.0 + 100.0, 190.0 + 27.0);
        state.send_slider(&mut audio, effects);
        assert_eq!(audio.sfx_volume, hundredths_gain(-620));
        // The thumb stays at the pointer until the window opens again.
        assert_eq!(state.thumb_centre(&audio, 1), 100.0);
        state.set_origin(GameOptionsOrigin::CommandCenter);
        assert_eq!(state.thumb_centre(&audio, 1), 96.0);
    }

    #[test]
    fn a_click_that_does_not_move_the_thumb_sends_the_opened_percent() {
        // Class_0066e010_vt14 posts +0xdc on the release, which
        // FUN_00604a50 derived from the opened thumb: a volume that reads
        // back as 48 opens at x 96 and sends floor(9600 / 201) = 47.
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        let mut audio = AudioVolumeState {
            music_volume: hundredths_gain(-620),
            ..Default::default()
        };
        let music = GameOptionsControl::MusicVolume;
        state.move_thumb(music, 392.0 + 3.0, 130.0 + 20.0);
        state.send_slider(&mut audio, music);
        assert_eq!(audio.music_volume, hundredths_gain(slider_hundredths(47)));
        // Full volume opens at 201, which sends 100: no change.
        let mut full = AudioVolumeState::default();
        state.set_origin(GameOptionsOrigin::CommandCenter);
        state.send_slider(&mut full, music);
        assert_eq!(full.music_volume, 1.0);
        assert!(!full.dirty);
    }

    #[test]
    fn a_slider_takes_a_press_anywhere_strictly_inside_its_window() {
        // FUN_006048f0: 0 < x < 201 and 0 < y < 54 in slider pixels.
        let music = GameOptionsControl::MusicVolume;
        assert!(inside_slider(music, 393.0, 131.0));
        assert!(inside_slider(music, 592.9, 183.9));
        assert!(!inside_slider(music, 392.5, 150.0));
        assert!(!inside_slider(music, 500.0, 184.0));
        assert!(!inside_slider(GameOptionsControl::Music, 500.0, 150.0));
        // The edges themselves are outside: x 201 and y 0.
        assert!(!inside_slider(music, 593.0, 150.0));
        assert!(!inside_slider(music, 500.0, 130.0));
    }

    #[test]
    fn a_held_move_inside_the_pressed_slider_moves_and_sends_and_nothing_else_does() {
        // Class_0066e010_vt14: only with the button down, the pointer
        // strictly inside (FUN_006048f0) and a slider pressed.
        let music = GameOptionsControl::MusicVolume;
        let inside = (392.0 + 100.0, 150.0);
        let sent = |pressed, pointer, down| {
            let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
            let mut audio = AudioVolumeState::default();
            state.pressed = pressed;
            state.held_slider_move(&mut audio, pointer, down);
            audio.music_volume
        };
        let moved = hundredths_gain(slider_hundredths(thumb_percent(100.0)));
        assert_eq!(sent(Some(music), inside, true), moved);
        assert_eq!(sent(Some(music), inside, false), 1.0);
        assert_eq!(sent(Some(music), (392.0 + 100.0, 190.0), true), 1.0);
        assert_eq!(sent(Some(GameOptionsControl::Music), inside, true), 1.0);
        assert_eq!(sent(None, inside, true), 1.0);
    }

    #[test]
    fn a_press_goes_to_a_slider_anywhere_and_to_other_controls_by_their_mask() {
        let music = GameOptionsControl::MusicVolume;
        let other = GameOptionsControl::Music;
        assert_eq!(press_target(Some(music), || None), Some(music));
        assert_eq!(press_target(Some(other), || None), None);
        assert_eq!(press_target(Some(other), || Some(other)), Some(other));
        assert_eq!(press_target(None, || Some(other)), Some(other));
    }

    #[test]
    fn a_full_volume_thumb_is_clipped_at_the_tracks_end() {
        // FUN_00604a50 clips the 11-wide thumb to the 201-pixel track: at
        // 100% (centre 201) five columns show (A0 200-options: the music
        // thumb cut at the right end).
        assert_eq!(thumb_span(201.0, 11.0), (196.0, 5.0));
        assert_eq!(thumb_span(100.0, 11.0), (95.0, 11.0));
        assert_eq!(thumb_span(300.0, 11.0), (295.0, 0.0));
    }

    #[test]
    fn a_negative_or_unreadable_amplitude_is_directsounds_minimum() {
        assert_eq!(gain_hundredths(-0.5), -10_000);
        assert_eq!(gain_hundredths(f32::NAN), -10_000);
        assert_eq!(gain_hundredths(0.0), -10_000);
    }

    #[test]
    fn only_a_slider_counts_as_the_pressed_slider() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        state.pressed = Some(GameOptionsControl::SoundVolume);
        assert_eq!(state.pressed_slider(), Some(GameOptionsControl::SoundVolume));
        state.pressed = Some(GameOptionsControl::Music);
        assert_eq!(state.pressed_slider(), None);
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
