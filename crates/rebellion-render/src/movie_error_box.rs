//! The video manager's box for a movie it cannot open.
//!
//! `FUN_005f6920` asks `FUN_005f6be0` (Smacker: `FUN_005f71f0`, `SmackOpen`)
//! for the movie's size; when that fails it calls `FUN_0060a9e0(path,
//! "PathName unknown to VideoManager")`, which is `MessageBeep(0x10)` and
//! `MessageBoxA(NULL, path, caption, 0x1010)`: an OK-only, system-modal box
//! with the stop icon (`MB_SYSTEMMODAL | MB_ICONHAND`). `MessageBoxA` returns
//! only when the box is answered; then `FUN_005f6920` posts 0x403 to the
//! movie window, whose handler (`FUN_00421790`, shared with 0x402, the
//! natural end) stops, posts 0x405 and calls the movie's callback: the flow
//! goes on exactly as after a movie that played to its end.
//!
//! The box itself is drawn by Windows, not by the game. hyp: its look here
//! (a Windows 10 message box at 96 DPI over the 640 by 480 client: white
//! title and body, grey button band, OK on the right) is not compared with
//! a capture.

use egui_macroquad::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// The caption (`s_PathName_unknown_to_VideoManager_006ac244`, file offset
/// 0x2aa044 in REBEXE.EXE).
pub const CAPTION: &str = "PathName unknown to VideoManager";

/// An input the box answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoxInput {
    /// Enter, the default button's key.
    EnterPressed,
    /// Escape: an OK-only box answers it as OK (IDOK).
    EscapePressed,
    /// Space released while the OK button has the focus.
    SpaceReleased,
    /// The left button went down at a point of the 640 by 480 client.
    MouseDown(f32, f32),
    /// The left button came up at a point of the 640 by 480 client.
    MouseUp(f32, f32),
}

/// The open box: the path it shows and whether the left button went down on
/// its OK button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieErrorBox {
    path: String,
    ok_pressed: bool,
}

/// Where the box's parts stand in the 640 by 480 client.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxLayout {
    pub frame: Rect,
    pub title: Rect,
    pub icon: Rect,
    pub text: Rect,
    pub band: Rect,
    pub ok: Rect,
}

// hyp: Windows 10 message-box metrics at 96 DPI.
const TITLE_HEIGHT: f32 = 31.0;
const MARGIN: f32 = 24.0;
const ICON: f32 = 32.0;
const ICON_GAP: f32 = 12.0;
const BAND_HEIGHT: f32 = 49.0;
const OK_SIZE: Vec2 = Vec2::new(88.0, 26.0);
const MIN_WIDTH: f32 = 180.0;
const TEXT_SIZE: f32 = 12.0;

impl MovieErrorBox {
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ok_pressed: false,
        }
    }

    /// The text the box shows: the movie's path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The box's parts for a text `text_width` pixels wide and a caption
    /// `caption_width` wide, centred in the 640 by 480 client.
    #[must_use]
    pub fn layout(text_width: f32, caption_width: f32) -> BoxLayout {
        let body = MARGIN + ICON + ICON_GAP + text_width + MARGIN;
        let width = body.max(caption_width + 2.0 * MARGIN + 46.0).max(MIN_WIDTH);
        let height = TITLE_HEIGHT + MARGIN + ICON + MARGIN + BAND_HEIGHT;
        let frame = Rect::from_center_size(Pos2::new(320.0, 240.0), Vec2::new(width, height));
        let title = Rect::from_min_size(frame.min, Vec2::new(width, TITLE_HEIGHT));
        let icon = Rect::from_min_size(
            Pos2::new(frame.min.x + MARGIN, title.max.y + MARGIN),
            Vec2::splat(ICON),
        );
        let text = Rect::from_min_max(
            Pos2::new(icon.max.x + ICON_GAP, icon.min.y),
            Pos2::new(frame.max.x - MARGIN, icon.max.y),
        );
        let band = Rect::from_min_max(Pos2::new(frame.min.x, frame.max.y - BAND_HEIGHT), frame.max);
        let ok = Rect::from_center_size(
            Pos2::new(band.max.x - 11.0 - OK_SIZE.x / 2.0, band.center().y),
            OK_SIZE,
        );
        BoxLayout {
            frame,
            title,
            icon,
            text,
            band,
            ok,
        }
    }

    /// Answer one input; `true` when it closes the box (IDOK). Only OK
    /// closes it: a click elsewhere does nothing, and a button released
    /// off OK after going down on it does not press it.
    pub fn handle(&mut self, input: BoxInput, layout: &BoxLayout) -> bool {
        match input {
            BoxInput::EnterPressed | BoxInput::EscapePressed | BoxInput::SpaceReleased => true,
            BoxInput::MouseDown(x, y) => {
                self.ok_pressed = layout.ok.contains(Pos2::new(x, y));
                false
            }
            BoxInput::MouseUp(x, y) => {
                let pressed = std::mem::take(&mut self.ok_pressed);
                pressed && layout.ok.contains(Pos2::new(x, y))
            }
        }
    }

    /// Whether the left button is held on OK (the button shows pressed).
    #[must_use]
    pub const fn ok_pressed(&self) -> bool {
        self.ok_pressed
    }

    /// Paint the box over the client whose top left is `origin`, scaled by
    /// `scale`; returns its layout.
    pub fn paint(&self, painter: &egui::Painter, origin: Pos2, scale: f32) -> BoxLayout {
        let font = FontId::proportional(TEXT_SIZE * scale);
        let measure = |text: &str| {
            painter
                .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
                .size()
                .x
                / scale
        };
        let layout = Self::layout(measure(&self.path), measure(CAPTION));
        let at = |rect: Rect| {
            Rect::from_min_max(
                origin + rect.min.to_vec2() * scale,
                origin + rect.max.to_vec2() * scale,
            )
        };
        let border = Color32::from_rgb(0x00, 0x78, 0xd7);
        painter.rect_filled(at(layout.frame), 0.0, Color32::WHITE);
        painter.rect_stroke(
            at(layout.frame),
            0.0,
            Stroke::new(scale, border),
            egui::StrokeKind::Inside,
        );
        painter.text(
            at(layout.title).left_center() + Vec2::new(10.0 * scale, 0.0),
            Align2::LEFT_CENTER,
            CAPTION,
            font.clone(),
            Color32::BLACK,
        );
        // The stop icon (MB_ICONHAND): a red disc with a white cross.
        let icon = at(layout.icon);
        painter.circle_filled(
            icon.center(),
            icon.width() / 2.0,
            Color32::from_rgb(0xe8, 0x11, 0x23),
        );
        let arm = icon.width() * 0.2;
        let cross = Stroke::new(2.5 * scale, Color32::WHITE);
        for (dx, dy) in [(arm, arm), (arm, -arm)] {
            painter.line_segment(
                [
                    icon.center() - Vec2::new(dx, dy),
                    icon.center() + Vec2::new(dx, dy),
                ],
                cross,
            );
        }
        painter.text(
            at(layout.text).left_center(),
            Align2::LEFT_CENTER,
            &self.path,
            font.clone(),
            Color32::BLACK,
        );
        painter.rect_filled(at(layout.band), 0.0, Color32::from_rgb(0xf0, 0xf0, 0xf0));
        let ok = at(layout.ok);
        let face = if self.ok_pressed {
            Color32::from_rgb(0xcc, 0xe4, 0xf7)
        } else {
            Color32::from_rgb(0xe1, 0xe1, 0xe1)
        };
        painter.rect_filled(ok, 0.0, face);
        // The default button carries the focus border.
        painter.rect_stroke(
            ok,
            0.0,
            Stroke::new(2.0 * scale, border),
            egui::StrokeKind::Inside,
        );
        painter.text(
            ok.center(),
            Align2::CENTER_CENTER,
            "OK",
            font,
            Color32::BLACK,
        );
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> BoxLayout {
        MovieErrorBox::layout(200.0, 190.0)
    }

    #[test]
    fn the_box_shows_the_originals_caption() {
        // s_PathName_unknown_to_VideoManager_006ac244 (FUN_005f6920).
        assert_eq!(CAPTION, "PathName unknown to VideoManager");
        assert_eq!(
            MovieErrorBox::new("data/video/105.webm").path(),
            "data/video/105.webm"
        );
    }

    #[test]
    fn enter_escape_and_space_answer_the_ok_only_box() {
        // MessageBoxA(.., 0x1010): MB_OK, so Escape too returns IDOK.
        let layout = layout();
        for input in [
            BoxInput::EnterPressed,
            BoxInput::EscapePressed,
            BoxInput::SpaceReleased,
        ] {
            assert!(MovieErrorBox::new("x").handle(input, &layout), "{input:?}");
        }
    }

    #[test]
    fn only_a_click_on_ok_closes_the_box() {
        // A system-modal box takes no click outside its OK button.
        let layout = layout();
        let ok = layout.ok.center();
        let mut dialog = MovieErrorBox::new("x");
        assert!(!dialog.handle(BoxInput::MouseDown(5.0, 5.0), &layout));
        assert!(!dialog.handle(BoxInput::MouseUp(5.0, 5.0), &layout));
        assert!(!dialog.handle(
            BoxInput::MouseDown(layout.text.center().x, layout.text.center().y),
            &layout
        ));
        assert!(!dialog.handle(BoxInput::MouseUp(ok.x, ok.y), &layout));
        // Down on OK and up off it presses nothing.
        assert!(!dialog.handle(BoxInput::MouseDown(ok.x, ok.y), &layout));
        assert!(dialog.ok_pressed());
        assert!(!dialog.handle(BoxInput::MouseUp(5.0, 5.0), &layout));
        assert!(!dialog.ok_pressed());
        assert!(!dialog.handle(BoxInput::MouseDown(ok.x, ok.y), &layout));
        assert!(dialog.handle(BoxInput::MouseUp(ok.x, ok.y), &layout));
    }

    #[test]
    fn the_box_is_centred_and_ok_sits_in_its_button_band() {
        let layout = layout();
        assert_eq!(layout.frame.center(), Pos2::new(320.0, 240.0));
        assert!(layout.frame.contains_rect(layout.ok));
        assert!(layout.band.contains_rect(layout.ok));
        assert!(layout.text.width() >= 200.0);
        let long = MovieErrorBox::layout(600.0, 190.0);
        assert!(long.text.width() >= 600.0);
    }
}
