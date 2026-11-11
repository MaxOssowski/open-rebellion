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
//! The box itself is drawn by Windows, not by the game. Its look here is
//! measured on A0 `2026-11-11-icon-menu-topic/53` (Windows 10 under the
//! game's 640 by 480, 256-colour mode): a classic 3D frame, a flat light
//! blue title, a white body with the stop icon and the path, and a cream
//! band with OK, 300 by 149 for the path
//! `C:\Games\Rebellion\MDATA\MDATA.005`.

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

// A0 2026-11-11-icon-menu-topic/53, in client pixels from the frame's
// top left: frame 300 by 149 at (173, 171); title (3, 3)..(297, 25); a
// close box (275, 5)..(295, 23); body (3, 26)..(297, 104); the icon at
// (24, 49), 32 square; the text from x 66; band (3, 104)..(297, 147); OK
// 75 by 23 at y 114, 18 in from the frame's right.
const FRAME: f32 = 3.0;
const HEIGHT: f32 = 149.0;
const TITLE_BOTTOM: f32 = 25.0;
const BODY_BOTTOM: f32 = 104.0;
const ICON_AT: Vec2 = Vec2::new(24.0, 49.0);
const ICON: f32 = 32.0;
const TEXT_X: f32 = 66.0;
/// The frame's width beyond the text: 300 less A0's 200-pixel path.
const TEXT_EXTRA: f32 = 100.0;
const CAPTION_X: f32 = 28.0;
/// The close box, 20 by 18, 5 in from the frame's right and 5 down.
const CLOSE_SIZE: Vec2 = Vec2::new(20.0, 18.0);
const CLOSE_INSET: f32 = 5.0;
const OK_SIZE: Vec2 = Vec2::new(75.0, 23.0);
const OK_TOP: f32 = 114.0;
const OK_RIGHT: f32 = 18.0;
/// A0's frame centre: (173 + 300 / 2, 171 + 149 / 2) less half a pixel
/// for the exclusive edge. hyp: a longer path keeps this centre.
const CENTRE: Pos2 = Pos2::new(323.0, 245.5);
const TEXT_SIZE: f32 = 12.0;

const FACE: Color32 = Color32::from_rgb(255, 251, 240);
const LIGHT: Color32 = Color32::WHITE;
const SHADOW: Color32 = Color32::from_rgb(160, 160, 164);
const DARK: Color32 = Color32::from_rgb(128, 128, 128);
const TITLE: Color32 = Color32::from_rgb(166, 202, 240);

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
    /// `caption_width` wide, at A0's place in the 640 by 480 client.
    #[must_use]
    pub fn layout(text_width: f32, caption_width: f32) -> BoxLayout {
        let width = (text_width + TEXT_EXTRA)
            .max(CAPTION_X + caption_width + CLOSE_INSET + CLOSE_SIZE.x + 4.0)
            .round();
        let frame = Rect::from_center_size(CENTRE, Vec2::new(width, HEIGHT));
        let at = |x: f32, y: f32| frame.min + Vec2::new(x, y);
        let title = Rect::from_min_max(at(FRAME, FRAME), at(width - FRAME, TITLE_BOTTOM));
        let icon = Rect::from_min_size(at(ICON_AT.x, ICON_AT.y), Vec2::splat(ICON));
        let text = Rect::from_min_max(
            Pos2::new(frame.min.x + TEXT_X, icon.min.y),
            Pos2::new(frame.min.x + TEXT_X + text_width, icon.max.y),
        );
        let band = Rect::from_min_max(at(FRAME, BODY_BOTTOM), at(width - FRAME, HEIGHT - 2.0));
        let ok = Rect::from_min_size(at(width - OK_RIGHT - OK_SIZE.x, OK_TOP), OK_SIZE);
        BoxLayout {
            frame,
            title,
            icon,
            text,
            band,
            ok,
        }
    }

    /// The title's close box in `frame`: A0 (448, 176)..(467, 193) in the
    /// frame (173, 171)..(472, 319).
    #[must_use]
    pub fn close_box(frame: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                frame.max.x - CLOSE_INSET - CLOSE_SIZE.x,
                frame.min.y + CLOSE_INSET,
            ),
            CLOSE_SIZE,
        )
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
        let frame = layout.frame;
        // The raised frame: face, then light inside face on the top and
        // left; shadow and dark on the right and bottom.
        painter.rect_filled(at(frame), 0.0, FACE);
        bevel(painter, &at, frame, [FACE, LIGHT], [DARK, SHADOW]);
        painter.rect_filled(at(layout.title), 0.0, TITLE);
        painter.text(
            at(layout.title).left_center() + Vec2::new((CAPTION_X - FRAME) * scale, 0.0),
            Align2::LEFT_CENTER,
            CAPTION,
            font.clone(),
            Color32::BLACK,
        );
        let close = Self::close_box(frame);
        painter.rect_filled(at(close), 0.0, FACE);
        bevel(painter, &at, close, [LIGHT, FACE], [DARK, SHADOW]);
        let mark = at(close).center();
        let arm = 4.0 * scale;
        for (dx, dy) in [(arm, arm), (arm, -arm)] {
            painter.line_segment(
                [mark - Vec2::new(dx, dy), mark + Vec2::new(dx, dy)],
                Stroke::new(2.0 * scale, Color32::BLACK),
            );
        }
        let body = Rect::from_min_max(
            Pos2::new(layout.title.min.x, layout.title.max.y + 1.0),
            Pos2::new(layout.title.max.x, layout.band.min.y),
        );
        painter.rect_filled(at(body), 0.0, LIGHT);
        stop_icon(painter, at(layout.icon), scale);
        painter.text(
            at(layout.text).left_center(),
            Align2::LEFT_CENTER,
            &self.path,
            font.clone(),
            Color32::BLACK,
        );
        painter.rect_filled(at(layout.band), 0.0, FACE);
        // The default OK: a dark outline, then light top and left, dark
        // and shadow bottom and right; pressed, it sinks.
        let ok = layout.ok;
        painter.rect_stroke(
            at(ok),
            0.0,
            Stroke::new(scale, DARK),
            egui::StrokeKind::Inside,
        );
        let inner = ok.shrink(1.0);
        if self.ok_pressed {
            painter.rect_stroke(
                at(inner),
                0.0,
                Stroke::new(scale, SHADOW),
                egui::StrokeKind::Inside,
            );
        } else {
            bevel(painter, &at, inner, [LIGHT, FACE], [DARK, SHADOW]);
        }
        let shift = if self.ok_pressed { scale } else { 0.0 };
        painter.text(
            at(ok).center() + Vec2::splat(shift),
            Align2::CENTER_CENTER,
            "OK",
            font,
            Color32::BLACK,
        );
        layout
    }
}

/// A two-pixel 3D edge around `rect`: `lit` (outer, inner) on the top and
/// left, `shade` (outer, inner) on the right and bottom.
fn bevel(
    painter: &egui::Painter,
    at: &impl Fn(Rect) -> Rect,
    rect: Rect,
    lit: [Color32; 2],
    shade: [Color32; 2],
) {
    for (inset, (light, dark)) in [(0.0, (lit[0], shade[0])), (1.0, (lit[1], shade[1]))] {
        let r = rect.shrink(inset);
        let line = |painter: &egui::Painter, rect: Rect, colour| {
            painter.rect_filled(at(rect), 0.0, colour);
        };
        line(
            painter,
            Rect::from_min_size(r.min, Vec2::new(r.width(), 1.0)),
            light,
        );
        line(
            painter,
            Rect::from_min_size(r.min, Vec2::new(1.0, r.height())),
            light,
        );
        line(
            painter,
            Rect::from_min_max(Pos2::new(r.min.x, r.max.y - 1.0), r.max),
            dark,
        );
        line(
            painter,
            Rect::from_min_max(Pos2::new(r.max.x - 1.0, r.min.y), r.max),
            dark,
        );
    }
}

/// The stop icon (`MB_ICONHAND`) as A0 draws it in 256 colours: a grey
/// ring, a red disc darker to the lower right, and a light grey cross.
fn stop_icon(painter: &egui::Painter, icon: Rect, scale: f32) {
    let centre = icon.center();
    let radius = icon.width() / 2.0;
    painter.circle_filled(centre, radius, DARK);
    painter.circle_filled(centre, radius - scale, LIGHT);
    painter.circle_filled(
        centre,
        radius - 2.0 * scale,
        Color32::from_rgb(192, 192, 192),
    );
    painter.circle_filled(centre, radius - 3.0 * scale, Color32::from_rgb(128, 0, 0));
    painter.circle_filled(
        centre - Vec2::splat(1.5 * scale),
        radius - 5.0 * scale,
        Color32::from_rgb(255, 0, 0),
    );
    let arm = radius * 0.45;
    for (dx, dy) in [(arm, arm), (arm, -arm)] {
        painter.line_segment(
            [
                centre - Vec2::new(dx, dy) + Vec2::splat(scale),
                centre + Vec2::new(dx, dy) + Vec2::splat(scale),
            ],
            Stroke::new(4.0 * scale, DARK),
        );
        painter.line_segment(
            [centre - Vec2::new(dx, dy), centre + Vec2::new(dx, dy)],
            Stroke::new(4.0 * scale, Color32::from_rgb(192, 192, 192)),
        );
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
    fn the_box_takes_a0s_rects_for_a0s_path() {
        // A0 2026-11-11-icon-menu-topic/53: the path's ink is 200 pixels
        // wide (x 239..438), the frame (173, 171)..(472, 319), title y
        // 174..195, band y 275..317, OK (380, 285)..(454, 307), the icon's
        // ring (197, 220)..(228, 251).
        let layout = MovieErrorBox::layout(200.0, 202.0);
        assert_eq!(
            layout.frame,
            Rect::from_min_max(Pos2::new(173.0, 171.0), Pos2::new(473.0, 320.0))
        );
        assert_eq!(
            layout.title,
            Rect::from_min_max(Pos2::new(176.0, 174.0), Pos2::new(470.0, 196.0))
        );
        assert_eq!(
            layout.band,
            Rect::from_min_max(Pos2::new(176.0, 275.0), Pos2::new(470.0, 318.0))
        );
        assert_eq!(
            layout.ok,
            Rect::from_min_max(Pos2::new(380.0, 285.0), Pos2::new(455.0, 308.0))
        );
        assert_eq!(
            layout.icon,
            Rect::from_min_max(Pos2::new(197.0, 220.0), Pos2::new(229.0, 252.0))
        );
        assert_eq!(layout.text.min.x, 239.0);
        assert_eq!(
            MovieErrorBox::close_box(layout.frame),
            Rect::from_min_max(Pos2::new(448.0, 176.0), Pos2::new(468.0, 194.0))
        );
        let short = MovieErrorBox::layout(120.0, 202.0);
        let close = MovieErrorBox::close_box(short.frame);
        assert!(short.title.contains_rect(close));
        assert_eq!(short.frame.max.x - close.max.x, 5.0);
        assert!(layout.band.contains_rect(layout.ok));
        let long = MovieErrorBox::layout(600.0, 202.0);
        assert!(long.text.width() >= 600.0);
        assert!(long.frame.contains_rect(long.text));
    }

    #[test]
    fn a_caption_wider_than_the_text_widens_the_box_past_its_close_box() {
        // The caption starts 28 in and the close box takes the title's last
        // 25 pixels; 4 more keep them apart.
        let layout = MovieErrorBox::layout(10.0, 400.0);
        assert_eq!(layout.frame.width(), 457.0);
    }
}
