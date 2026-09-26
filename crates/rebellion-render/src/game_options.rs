//! Original bitmap-driven unified Game Options surface.

use macroquad::prelude::*;

use crate::audio::AudioVolumeState;
use crate::bmp_cache::{resources, BmpCache, DllSource};
use crate::panels::SaveSlotInfo;

const LOGICAL_WIDTH: f32 = 640.0;
const LOGICAL_HEIGHT: f32 = 480.0;

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
    Delete { slot: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

const MUSIC_RECT: NativeRect = NativeRect::new(351.0, 76.0, 19.0, 35.0);
const MUSIC_SLIDER_RECT: NativeRect = NativeRect::new(391.0, 130.0, 212.0, 54.0);
const SOUND_SLIDER_RECT: NativeRect = NativeRect::new(391.0, 192.0, 212.0, 54.0);
const TACTICAL_TOGGLE_RECTS: [NativeRect; 5] = [
    NativeRect::new(359.0, 310.0, 35.0, 22.0),
    NativeRect::new(359.0, 337.0, 35.0, 22.0),
    NativeRect::new(359.0, 364.0, 35.0, 22.0),
    NativeRect::new(359.0, 391.0, 35.0, 22.0),
    NativeRect::new(359.0, 418.0, 35.0, 22.0),
];
const RESTART_RECT: NativeRect = NativeRect::new(77.0, 383.0, 42.0, 42.0);
const RETURN_RECT: NativeRect = NativeRect::new(164.0, 383.0, 42.0, 42.0);
const EXIT_RECT: NativeRect = NativeRect::new(251.0, 383.0, 42.0, 42.0);

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
}

impl Default for GameOptionsState {
    fn default() -> Self {
        Self {
            pending: None,
            occupied: [false; 10],
            names: std::array::from_fn(|slot| format!("Saved Game {}", slot + 1)),
            suspended: false,
            error: None,
            origin: GameOptionsOrigin::ShuttleCockpit,
            pressed: None,
            show_starfield: true,
            show_planet: true,
            show_pyrotechnics: true,
            high_detail_models: true,
            display_holocube: true,
        }
    }
}

impl GameOptionsState {
    pub fn refresh_saves(&mut self, saves: &[SaveSlotInfo]) {
        self.occupied.fill(false);
        self.names = std::array::from_fn(|slot| format!("Saved Game {}", slot + 1));
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
            GameOptionsAction::Save { slot, name } => {
                self.origin == GameOptionsOrigin::CommandCenter
                    && *slot < self.occupied.len()
                    && !name.trim().is_empty()
            }
            GameOptionsAction::Load { slot } | GameOptionsAction::Delete { slot } => {
                self.origin != GameOptionsOrigin::TacticalBattle
                    && self.occupied.get(*slot).copied().unwrap_or(false)
            }
            GameOptionsAction::Restart => self.origin == GameOptionsOrigin::CommandCenter,
            GameOptionsAction::Return | GameOptionsAction::Exit => true,
            GameOptionsAction::None => false,
        }
    }

    pub fn request(&mut self, action: GameOptionsAction) -> GameOptionsAction {
        if self.suspended || self.pending.is_some() || !self.enabled(&action) {
            return GameOptionsAction::None;
        }
        let confirm = match &action {
            GameOptionsAction::Save { slot, .. } => self.occupied[*slot],
            GameOptionsAction::Load { .. } => self.origin.has_live_campaign(),
            GameOptionsAction::Restart
            | GameOptionsAction::Exit
            | GameOptionsAction::Delete { .. } => true,
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
        if self.pending.take().is_some() {
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

fn save_rect(slot: usize) -> NativeRect {
    NativeRect::new(35.0, 81.0 + slot as f32 * 42.0, 42.0, 20.0)
}

fn load_rect(slot: usize) -> NativeRect {
    NativeRect::new(285.0, 81.0 + slot as f32 * 42.0, 41.0, 20.0)
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
    if origin == GameOptionsOrigin::CommandCenter && RESTART_RECT.contains(x, y) {
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
            resources::common::OPTIONS_TOGGLE_ON,
            TACTICAL_TOGGLE_RECTS[index],
        ),
        GameOptionsControl::Restart => (resources::common::BTN_RESTART_GAME_NORMAL, RESTART_RECT),
        GameOptionsControl::Return => (
            resources::common::BTN_RETURN_COMMAND_CENTER_NORMAL,
            RETURN_RECT,
        ),
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

fn draw_label(canvas: OptionsCanvas, text: &str, x: f32, y: f32, size: f32) {
    let (screen_x, screen_y) = canvas.point(x, y);
    draw_text(
        text,
        screen_x,
        screen_y,
        size * canvas.scale,
        Color::from_rgba(0, 255, 24, 255),
    );
}

fn draw_volume(cache: &mut BmpCache, canvas: OptionsCanvas, y: f32, value: f32, alternate: bool) {
    draw_bitmap(
        cache,
        canvas,
        if alternate {
            resources::common::OPTIONS_VOLUME_RAIL_ALT
        } else {
            resources::common::OPTIONS_VOLUME_RAIL
        },
        391.0,
        y,
    );
    let handle_x = 392.0 + value.clamp(0.0, 1.0) * 199.0;
    draw_bitmap(
        cache,
        canvas,
        resources::common::OPTIONS_VOLUME_HANDLE,
        handle_x,
        y + 4.0,
    );
}

fn set_slider(audio: &mut AudioVolumeState, control: GameOptionsControl, logical_x: f32) {
    let value = ((logical_x - 396.0) / 199.0).clamp(0.0, 1.0);
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
    draw_bitmap(cache, canvas, resources::common::GAME_OPTIONS_BG, 0.0, 0.0);

    draw_label(canvas, "Saved Games", 132.0, 54.0, 17.0);
    draw_label(canvas, "Sound Options", 438.0, 54.0, 17.0);
    draw_label(canvas, "Play Music", 376.0, 100.0, 17.0);
    draw_label(
        canvas,
        if audio.music_enabled() { "On" } else { "Off" },
        580.0,
        100.0,
        17.0,
    );
    draw_label(canvas, "Tactical Display Options", 377.0, 289.0, 17.0);
    for (index, label) in [
        "Show Starfield",
        "Show Planet",
        "Show Pyrotechnics",
        "Use High Detail Models",
        "Display Holocube",
    ]
    .into_iter()
    .enumerate()
    {
        let y = 329.0 + index as f32 * 27.0;
        draw_label(canvas, label, 395.0, y, 16.0);
        draw_label(
            canvas,
            if state.tactical_flags()[index] {
                "On"
            } else {
                "Off"
            },
            580.0,
            y,
            16.0,
        );
    }
    draw_label(canvas, "Version: 1.0.0 Open Rebellion", 84.0, 454.0, 11.0);

    let pointer = canvas.logical_pointer();
    let held = state
        .pressed
        .filter(|_| is_mouse_button_down(MouseButton::Left));
    draw_bitmap(
        cache,
        canvas,
        if !audio.backend_available {
            resources::common::OPTIONS_MUSIC_DISABLED
        } else if held == Some(GameOptionsControl::Music) {
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
        let resource = if !state.origin.tactical_toggles_enabled() {
            resources::common::OPTIONS_TOGGLE_OFF
        } else if held == Some(GameOptionsControl::Tactical(index)) {
            resources::common::OPTIONS_TOGGLE_PRESSED
        } else if state.tactical_flags()[index] {
            resources::common::OPTIONS_TOGGLE_ON
        } else {
            resources::common::OPTIONS_TOGGLE_OFF
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
        if state.origin != GameOptionsOrigin::CommandCenter {
            if let Some(save) = occupied {
                draw_label(canvas, &save.name, 116.0, 99.0 + slot as f32 * 42.0, 14.0);
            }
        }
    }

    draw_bitmap(
        cache,
        canvas,
        if !state.origin.has_live_campaign() {
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
        if !state.origin.has_live_campaign() {
            resources::common::BTN_RETURN_COMMAND_CENTER_DISABLED
        } else if held == Some(GameOptionsControl::Return) {
            resources::common::BTN_RETURN_COMMAND_CENTER_PRESSED
        } else {
            resources::common::BTN_RETURN_COMMAND_CENTER_NORMAL
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

    if state.pending.is_some() || state.suspended {
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
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
        // The area itself must not claim the entire canvas; only actual controls do.
        if state.origin == GameOptionsOrigin::CommandCenter {
            for slot in 0..6 {
                ui.add_enabled_ui(state.pending.is_none() && !state.suspended, |ui| {
                    ui.put(rect(116.0, 81.0 + slot as f32 * 42.0, 162.0, 20.0),
                        egui::TextEdit::singleline(&mut state.names[slot])
                            .id_source(("options_save_name", slot)).frame(false)
                            .font(FontId::monospace(12.0 * canvas.scale)).text_color(Color32::WHITE));
                });
            }
        }
        if let Some(action) = &state.pending {
            let assets = [10623, 10624, 10625, 10626, 10627];
            let complete = assets.iter().all(|id| cache.get(ctx, DllSource::Rebdlog, *id).is_some() && cache.original_resource_size(DllSource::Rebdlog, *id).is_some());
            let bounds = rect(114.0, 152.0, 412.0, 176.0);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            if complete {
                ui.painter().image(cache.get(ctx, DllSource::Rebdlog, 10623).unwrap().id(), bounds, uv, Color32::WHITE);
            } else {
                ui.painter().rect_filled(bounds, 4.0, Color32::from_rgb(30, 30, 35));
            }
            let message = match action {
                GameOptionsAction::Save { .. } => "Saving the selected game will destroy a previously saved game. Save anyway?",
                GameOptionsAction::Load { .. } => "Loading the selected game will destroy unsaved changes. Load without saving?",
                GameOptionsAction::Delete { .. } => "Delete the selected saved game?",
                GameOptionsAction::Restart => "Returning to the shuttle cockpit will cause unsaved changes to be lost. Return without saving?",
                GameOptionsAction::Exit => "Unsaved changes will be lost. Exit the game?",
                _ => "Return?",
            };
            let text = ui.painter().layout(message.into(), FontId::monospace(12.0 * canvas.scale), Color32::GREEN, 310.0 * canvas.scale);
            ui.painter().galley(rect(160.0, 177.0, 320.0, 75.0).center() - text.size() / 2.0, text, Color32::GREEN);
            for (x, normal, pressed, value, label) in [(252.0,10624,10625,true,"Yes"),(343.0,10626,10627,false,"No")] {
                let bounds = rect(x, 287.0, 57.0, 28.0);
                if complete {
                    let response = ui.interact(bounds, ui.id().with(normal), egui::Sense::click());
                    let id = if response.is_pointer_button_down_on() { pressed } else { normal };
                    ui.painter().image(cache.get(ctx, DllSource::Rebdlog, id).unwrap().id(), bounds, uv, Color32::WHITE);
                    let capture_id = ui.id().with((normal, "source_capture"));
                    if ctx.input(|i| i.pointer.primary_pressed()) {
                        let hit = ctx.pointer_latest_pos().is_some_and(|pos| bounds.contains(pos)
                            && cache.is_resource_hit(DllSource::Rebdlog, normal,
                                ((pos.x - bounds.left()) / canvas.scale) as usize,
                                ((pos.y - bounds.top()) / canvas.scale) as usize));
                        ui.data_mut(|data| data.insert_temp(capture_id, hit));
                    }
                    let captured = ui.data(|data| data.get_temp::<bool>(capture_id).unwrap_or(false));
                    if captured && response.clicked() && response.interact_pointer_pos().is_some_and(|pos| {
                        cache.is_resource_hit(DllSource::Rebdlog, normal,
                            ((pos.x - bounds.left()) / canvas.scale) as usize,
                            ((pos.y - bounds.top()) / canvas.scale) as usize)
                    }) { answer = Some(value); }
                } else {
                    let response = ui.put(bounds, egui::Button::new(label).min_size(bounds.size()));
                    if response.clicked() { answer = Some(value); }
                }
            }
        }
        if let Some(error) = &state.error {
            ui.painter().text(rect(25.0, 440.0, 590.0, 20.0).left_center(), Align2::LEFT_CENTER,
                error, FontId::monospace(11.0 * canvas.scale), Color32::LIGHT_RED);
        }
    });
    answer.map_or(GameOptionsAction::None, |accepted| state.confirm(accepted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occupied_legacy_slots_wait_for_confirmation_and_cancellation_preserves_them() {
        let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
        state.refresh_saves(&[SaveSlotInfo {
            slot: 9,
            name: "Legacy".into(),
            timestamp: String::new(),
            game_tick: 0,
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
        for (slot, name) in [(10, "Name"), (usize::MAX, "Name"), (0, " ")] {
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
    }

    #[test]
    fn missing_dialog_assets_still_allow_confirmation_and_cancellation() {
        use egui_macroquad::egui::{self, Pos2, Rect, Vec2};
        for accepted in [false, true] {
            let ctx = egui::Context::default();
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
            masked_control_at(&mut cache, origin, 355.0, 80.0),
            Some(GameOptionsControl::Music)
        );
        for (x, y) in [
            (351.0, 80.0),
            (355.0, 76.0),
            (370.0, 80.0),
            (355.0, 111.0),
            (352.0, 77.0),
        ] {
            assert_eq!(masked_control_at(&mut cache, origin, x, y), None, "{x},{y}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn original_hit_regions_reject_exact_outer_edges() {
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 351.0, 76.0),
            Some(GameOptionsControl::Music)
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 370.0, 76.0),
            None
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 35.0, 81.0),
            Some(GameOptionsControl::Save(0))
        );
        assert_eq!(
            control_at(GameOptionsOrigin::CommandCenter, 285.0, 291.0),
            Some(GameOptionsControl::Load(5))
        );
    }

    #[test]
    fn shuttle_and_tactical_contexts_disable_original_controls() {
        assert_eq!(
            control_at(GameOptionsOrigin::ShuttleCockpit, 80.0, 400.0),
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
    fn tactical_flags_default_to_original_on_state() {
        assert_eq!(GameOptionsState::default().tactical_flags(), [true; 5]);
    }
}
