//! The original's tooltips (`ghidra/notes/tooltips.md`).
//!
//! `FUN_006008e0` creates one shared `tooltips_class32` control for the
//! whole game, activates it (`TTM_ACTIVATE`) and sets its delays with
//! `TTM_SETDELAYTIME(TTDT_AUTOMATIC, 500)`: the tip shows after the pointer
//! rests 500 ms on a control, hides after 5000 ms, and moving from a shown
//! tip to another control shows the next after 100 ms. Each control adds
//! itself with `TTM_ADDTOOL` (`FUN_00600970`, `FUN_00600bd0`) and its text,
//! a TEXTSTRA string.
//!
//! The port keeps one tip for the frame. A control that the pointer is over
//! calls [`tool`] with its text while it paints; [`paint`] runs the timers
//! on the previous frame's control and draws the tip above everything.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use egui_macroquad::egui;

/// `TTDT_AUTOMATIC` 500: the initial delay.
pub const INITIAL_DELAY: f64 = 0.5;
/// `TTDT_AUTOMATIC` 500: the tip hides after ten times the initial delay.
pub const AUTOPOP_DELAY: f64 = 5.0;
/// `TTDT_AUTOMATIC` 500: the delay from one shown tip to the next, a fifth
/// of the initial delay.
pub const RESHOW_DELAY: f64 = 0.1;

/// How far below the pointer's hot spot the tip's top sits, in client
/// pixels; its left is the hot spot's. A0 `tooltip/01.png` and `03.png`:
/// the pointer at (175, 25) and (260, 25) puts the tip at (175, 46) and
/// (260, 46), below the arrow cursor.
pub const POINTER_DROP: f32 = 21.0;

/// The tip's height in client pixels (A0 `tooltip/01.png`: rows 46..=64).
pub const TIP_HEIGHT: f32 = 19.0;

/// The text's line box in the tip, in client pixels. A0: the ink starts 4
/// pixels in and the capitals 5 pixels down; egui's line box sits one pixel
/// left of its ink and three above the capitals.
const TEXT_INSET: (f32, f32) = (3.0, 2.0);

/// Room around the text: the tip is the text's advance plus five pixels
/// wide (A0: "Raw Materials Monitor" 124, "Refined Materials Monitor" 143).
const WIDTH_PADDING: f32 = 5.0;

/// The text's em in client pixels. The system font of A0 is Arial-metric;
/// an em of 12 gives the A0 widths within one pixel.
pub const FONT_EM: f32 = 12.0;

/// egui sizes a font by its cell, ascent plus descent (`theme::original_font`):
/// Arial's cell is 2288 of its 2048 units.
const FONT_SIZE: f32 = FONT_EM * 2288.0 / 2048.0;

/// The tip's fill, `COLOR_INFOBK` of A0 (Windows 10 in 256 colours).
pub const FILL: egui::Color32 = egui::Color32::from_rgb(255, 251, 240);
/// The tip's one-pixel border (A0).
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
/// The text, `COLOR_INFOTEXT`.
pub const TEXT: egui::Color32 = egui::Color32::BLACK;

/// The 640 by 480 client.
const CLIENT: (f32, f32) = (640.0, 480.0);

static STRINGS: LazyLock<Mutex<HashMap<u16, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Keep TEXTSTRA's string table for [`string`].
pub fn set_strings(strings: HashMap<u16, String>) {
    *STRINGS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = strings;
}

/// A tooltip text from TEXTSTRA's string table, if it was loaded.
#[must_use]
pub fn string(id: u16) -> Option<String> {
    STRINGS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&id)
        .cloned()
}

/// The tip's timers, from one frame to the next.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TipTimer {
    tool: Option<egui::Id>,
    text: String,
    /// When the pointer last came onto the tool or moved on it unshown.
    rest_since: f64,
    delay: f64,
    shown_at: Option<f64>,
    /// The pointer's client position when the tip showed.
    anchor: (f32, f32),
    /// A click or the autopop hid the tip until the pointer leaves.
    done: bool,
}

impl TipTimer {
    /// Advance to `now` with the control under the pointer. Returns the
    /// text and the pointer position the tip shows at, if it shows.
    pub fn step(
        &mut self,
        now: f64,
        hovered: Option<(egui::Id, &str)>,
        pointer: (f32, f32),
        moved: bool,
        pressed: bool,
    ) -> Option<(&str, (f32, f32))> {
        let Some((id, text)) = hovered else {
            *self = Self::default();
            return None;
        };
        if self.tool != Some(id) {
            let shown = self.shown_at.is_some() && !self.done;
            *self = Self {
                tool: Some(id),
                rest_since: now,
                delay: if shown { RESHOW_DELAY } else { INITIAL_DELAY },
                ..Self::default()
            };
        }
        if self.text != text {
            text.clone_into(&mut self.text);
        }
        if pressed {
            self.done = true;
            self.shown_at = None;
        }
        if self.done {
            return None;
        }
        match self.shown_at {
            None if moved => {
                self.rest_since = now;
                None
            }
            None if now - self.rest_since >= self.delay => {
                self.shown_at = Some(now);
                self.anchor = pointer;
                Some((&self.text, self.anchor))
            }
            None => None,
            Some(at) if now - at >= AUTOPOP_DELAY => {
                self.done = true;
                self.shown_at = None;
                None
            }
            Some(_) => Some((&self.text, self.anchor)),
        }
    }
}

/// The tip's rectangle in client pixels for a text `width` wide, shown with
/// the pointer at `pointer`. It stays one pixel inside the client's right
/// and bottom edges: past the right it moves left to end at 639 (A0
/// `tooltip/25-gid.png`, `27-rail-support.png`), past the bottom it goes
/// above the pointer, ending at the pointer's row (A0 `20-sysfinder.png`:
/// the pointer at y 440 puts the tip at 421..440).
#[must_use]
pub fn tip_rect(pointer: (f32, f32), width: f32) -> egui::Rect {
    let w = (width + WIDTH_PADDING).ceil();
    let mut x = pointer.0;
    let mut y = pointer.1 + POINTER_DROP;
    if x + w > CLIENT.0 - 1.0 {
        x = CLIENT.0 - 1.0 - w;
    }
    if y + TIP_HEIGHT > CLIENT.1 - 1.0 {
        y = pointer.1 - TIP_HEIGHT;
    }
    egui::Rect::from_min_size(
        egui::pos2(x.max(0.0), y.max(0.0)),
        egui::vec2(w, TIP_HEIGHT),
    )
}

#[derive(Clone, Default)]
struct Frame {
    hovered: Option<(egui::Id, String)>,
}

fn frame_id() -> egui::Id {
    egui::Id::new("original-tooltip-frame")
}

/// Register the control under the pointer this frame. Call it only while the
/// pointer is over the control and nothing covers it; the last call wins.
pub fn tool(ctx: &egui::Context, id: egui::Id, text: &str) {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Frame>(frame_id()).hovered = Some((id, text.to_owned()));
    });
}

/// [`tool`] for an egui response: registers it while hovered.
pub fn attach(response: &egui::Response, text: &str) {
    if response.hovered() {
        tool(&response.ctx, response.id, text);
    }
}

/// [`tool`] for a control painted on the background, such as the command
/// center's chrome: registers it while `enabled`, the pointer is inside
/// `screen_rect` and no window covers the pointer.
pub fn chrome_tool(
    ctx: &egui::Context,
    id: egui::Id,
    screen_rect: egui::Rect,
    text: &str,
    enabled: bool,
) {
    let over = ctx
        .input(|input| input.pointer.latest_pos())
        .is_some_and(|p| pt_in_rect(screen_rect, p));
    if enabled && over && !ctx.is_pointer_over_area() {
        tool(ctx, id, text);
    }
}

/// `PtInRect`: the right and bottom edges are outside.
fn pt_in_rect(rect: egui::Rect, p: egui::Pos2) -> bool {
    p.x >= rect.min.x && p.x < rect.max.x && p.y >= rect.min.y && p.y < rect.max.y
}

/// [`tool`] for a control with its own egui response: registers TEXTSTRA
/// `string` while the response is hovered (egui's hit test, so nothing
/// covers it) and the pointer is inside `rect` (`PtInRect`).
pub fn response_tool(response: &egui::Response, rect: egui::Rect, string: u16) {
    let inside = response
        .ctx
        .input(|input| input.pointer.latest_pos())
        .is_some_and(|p| pt_in_rect(rect, p));
    if response.hovered() && inside {
        if let Some(text) = self::string(string) {
            tool(&response.ctx, response.id, &text);
        }
    }
}

/// [`tool`] for a control of a window drawn in `ui`: registers TEXTSTRA
/// `string` while the pointer is inside `rect`, the window is the top layer
/// under it and the control is `enabled`. A disabled window shows no tip
/// (`EnableWindow(FALSE)`, `ghidra/notes/tooltips.md`).
pub fn window_tool(
    ui: &egui::Ui,
    rect: egui::Rect,
    id: impl std::hash::Hash,
    string: u16,
    enabled: bool,
) {
    let ctx = ui.ctx();
    let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) else {
        return;
    };
    if !enabled
        || !ui.is_enabled()
        || !pt_in_rect(rect, pointer)
        || ctx.layer_id_at(pointer) != Some(ui.layer_id())
    {
        return;
    }
    if let Some(text) = self::string(string) {
        tool(ctx, ui.id().with(("tooltip", id)), &text);
    }
}

/// Run the timers on the control the last frame registered and draw the tip
/// over the 640 by 480 client whose top left is `origin`, at `scale` screen
/// points per client pixel. Call once a frame, before the controls paint.
pub fn paint(ctx: &egui::Context, origin: egui::Pos2, scale: f32) {
    let hovered = ctx.data_mut(|data| {
        std::mem::take(&mut data.get_temp_mut_or_default::<Frame>(frame_id()).hovered)
    });
    let (now, pointer, moved, pressed) = ctx.input(|input| {
        (
            input.time,
            input.pointer.latest_pos(),
            input.pointer.delta() != egui::Vec2::ZERO,
            input.pointer.any_pressed(),
        )
    });
    let pointer = pointer.map_or((0.0, 0.0), |p| {
        ((p.x - origin.x) / scale, (p.y - origin.y) / scale)
    });
    let timer_id = egui::Id::new("original-tooltip-timer");
    let mut timer = ctx.data_mut(|data| data.get_temp::<TipTimer>(timer_id).unwrap_or_default());
    let shown = timer
        .step(
            now,
            hovered.as_ref().map(|(id, text)| (*id, text.as_str())),
            pointer,
            moved,
            pressed,
        )
        .map(|(text, at)| (text.to_owned(), at));
    ctx.data_mut(|data| data.insert_temp(timer_id, timer));
    let Some((text, at)) = shown else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Debug,
        egui::Id::new("original-tooltip"),
    ));
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(FONT_SIZE * scale), TEXT);
    let rect = tip_rect(at, galley.size().x / scale);
    let screen =
        egui::Rect::from_min_size(origin + rect.min.to_vec2() * scale, rect.size() * scale);
    painter.rect_filled(screen, 0.0, FILL);
    painter.rect_stroke(
        screen.shrink(scale * 0.5),
        0.0,
        egui::Stroke::new(scale, BORDER),
        egui::StrokeKind::Middle,
    );
    painter.galley(
        screen.min + egui::vec2(TEXT_INSET.0, TEXT_INSET.1) * scale,
        galley,
        TEXT,
    );
    // The timers run on frames: keep them coming while a tip waits or shows.
    ctx.request_repaint();
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// TEXTSTRA strings the tests use; every test that sets the global
    /// table sets all of them.
    pub(crate) const TEST_STRINGS: &[(u16, &str)] = &[
        (5392, "Raw Materials Monitor"),
        (5393, "Refined Materials Monitor"),
        (5394, "Maintenance Monitor"),
        (5395, "Game Speed Control"),
        (0x1501, "System Finder"),
        (0x1500, "Game Controls"),
        (0x173c, "Legend"),
        (0x8011, "Popular Support Messages"),
        (0x8015, "Chat Messages"),
    ];

    fn id(n: u32) -> egui::Id {
        egui::Id::new(n)
    }

    #[test]
    fn a_tip_shows_after_the_pointer_rests_half_a_second() {
        // FUN_006008e0: TTM_SETDELAYTIME(TTDT_AUTOMATIC, 500).
        let mut timer = TipTimer::default();
        assert_eq!(
            timer.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(0.49, Some((id(1), "Raw")), (175.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(0.5, Some((id(1), "Raw")), (175.0, 25.0), false, false),
            Some(("Raw", (175.0, 25.0)))
        );
    }

    #[test]
    fn moving_on_the_control_restarts_the_wait_but_not_a_shown_tip() {
        let mut timer = TipTimer::default();
        timer.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        timer.step(0.4, Some((id(1), "Raw")), (178.0, 25.0), true, false);
        assert_eq!(
            timer.step(0.8, Some((id(1), "Raw")), (178.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(0.9, Some((id(1), "Raw")), (178.0, 25.0), false, false),
            Some(("Raw", (178.0, 25.0)))
        );
        // A shown tip stays where it showed.
        assert_eq!(
            timer.step(1.0, Some((id(1), "Raw")), (190.0, 27.0), true, false),
            Some(("Raw", (178.0, 25.0)))
        );
    }

    #[test]
    fn a_tip_hides_after_five_seconds_until_the_pointer_leaves() {
        // A0 tooltip/04-4s.png shows the tip, 05-6s5.png and 06-moved.png do
        // not: TTDT_AUTOMATIC's autopop is ten times 500 ms.
        let mut timer = TipTimer::default();
        timer.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        assert!(timer
            .step(0.5, Some((id(1), "Raw")), (175.0, 25.0), false, false)
            .is_some());
        assert!(timer
            .step(5.4, Some((id(1), "Raw")), (175.0, 25.0), false, false)
            .is_some());
        assert_eq!(
            timer.step(5.5, Some((id(1), "Raw")), (175.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(7.0, Some((id(1), "Raw")), (185.0, 28.0), true, false),
            None
        );
        assert_eq!(
            timer.step(9.0, Some((id(1), "Raw")), (185.0, 28.0), false, false),
            None
        );
        timer.step(9.1, None, (300.0, 200.0), true, false);
        timer.step(9.2, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        assert!(timer
            .step(9.7, Some((id(1), "Raw")), (175.0, 25.0), false, false)
            .is_some());
    }

    #[test]
    fn a_click_hides_the_tip_until_the_pointer_leaves() {
        let mut timer = TipTimer::default();
        timer.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        assert!(timer
            .step(0.6, Some((id(1), "Raw")), (175.0, 25.0), false, false)
            .is_some());
        assert_eq!(
            timer.step(0.7, Some((id(1), "Raw")), (175.0, 25.0), false, true),
            None
        );
        assert_eq!(
            timer.step(2.0, Some((id(1), "Raw")), (175.0, 25.0), false, false),
            None
        );
    }

    #[test]
    fn the_next_control_shows_its_tip_after_a_tenth_of_a_second() {
        // TTDT_AUTOMATIC: the reshow delay is a fifth of the initial 500 ms.
        let mut timer = TipTimer::default();
        timer.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        assert!(timer
            .step(0.5, Some((id(1), "Raw")), (175.0, 25.0), false, false)
            .is_some());
        assert_eq!(
            timer.step(0.6, Some((id(2), "Refined")), (273.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(0.69, Some((id(2), "Refined")), (273.0, 25.0), false, false),
            None
        );
        assert_eq!(
            timer.step(0.71, Some((id(2), "Refined")), (273.0, 25.0), false, false),
            Some(("Refined", (273.0, 25.0)))
        );
        // Without a shown tip the next control waits the full delay.
        let mut cold = TipTimer::default();
        cold.step(0.0, Some((id(1), "Raw")), (175.0, 25.0), false, false);
        cold.step(0.2, Some((id(2), "Refined")), (273.0, 25.0), false, false);
        assert_eq!(
            cold.step(0.4, Some((id(2), "Refined")), (273.0, 25.0), false, false),
            None
        );
        assert!(cold
            .step(0.75, Some((id(2), "Refined")), (273.0, 25.0), false, false)
            .is_some());
    }

    #[test]
    fn a_new_text_on_the_same_control_shows_at_once() {
        // FUN_00600cc0 (TTM_UPDATETIPTEXT) changes a control's text in place.
        let mut timer = TipTimer::default();
        timer.step(0.0, Some((id(1), "Old")), (175.0, 25.0), false, false);
        assert!(timer
            .step(0.6, Some((id(1), "Old")), (175.0, 25.0), false, false)
            .is_some());
        assert_eq!(
            timer.step(0.7, Some((id(1), "New")), (175.0, 25.0), false, false),
            Some(("New", (175.0, 25.0)))
        );
    }

    #[test]
    fn the_text_is_twelve_pixels_to_the_em() {
        // Arial's cell is 2288 of its 2048 units: an em of 12 is egui size
        // 13.40625.
        assert!((FONT_SIZE - 13.406_25).abs() < 1e-4);
    }

    #[test]
    fn the_string_table_gives_the_tooltip_texts() {
        // Every test sets the same table: the strings are global.
        set_strings(
            TEST_STRINGS
                .iter()
                .map(|(id, text)| (*id, (*text).to_owned()))
                .collect(),
        );
        assert_eq!(string(5392).as_deref(), Some("Raw Materials Monitor"));
        assert_eq!(string(5391), None);
    }

    /// Run frames with the pointer at `pointer` (screen points) at the
    /// given times; `each` registers controls. Returns the last frame's
    /// shapes.
    pub(crate) fn frames(
        ctx: &egui::Context,
        pointer: egui::Pos2,
        times: &[f64],
        origin: egui::Pos2,
        scale: f32,
        mut each: impl FnMut(&egui::Context),
    ) -> Vec<egui::Shape> {
        let mut shapes = Vec::new();
        for (index, &time) in times.iter().enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1040.0),
                )),
                time: Some(time),
                events: if index == 0 {
                    vec![egui::Event::PointerMoved(pointer)]
                } else {
                    Vec::new()
                },
                ..Default::default()
            };
            shapes = ctx
                .run(input, |ctx| {
                    paint(ctx, origin, scale);
                    each(ctx);
                })
                .shapes
                .into_iter()
                .map(|clipped| clipped.shape)
                .collect();
        }
        shapes
    }

    pub(crate) fn tip_shapes(shapes: &[egui::Shape]) -> Option<(egui::Rect, egui::Pos2, String)> {
        let rect = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == FILL => Some(rect.rect),
            _ => None,
        })?;
        let (pos, text) = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains(' ') || !text.galley.job.text.is_empty() =>
            {
                Some((text.pos, text.galley.job.text.clone()))
            }
            _ => None,
        })?;
        Some((rect, pos, text))
    }

    #[test]
    fn the_tip_paints_over_the_client_below_the_pointer() {
        // A0 tooltip/01.png: the pointer at (175, 25), the tip's box at
        // (175, 46), 19 high, a grey border round the light fill, the text
        // 3 by 2 into the box's line.
        let ctx = egui::Context::default();
        let origin = egui::pos2(10.0, 20.0);
        let pointer = origin + egui::vec2(175.0, 25.0) * 2.0;
        let shapes = frames(&ctx, pointer, &[0.0, 0.1, 0.7], origin, 2.0, |ctx| {
            tool(ctx, egui::Id::new("raw"), "Raw Materials Monitor");
        });
        let (rect, text_pos, text) = tip_shapes(&shapes).expect("a tip");
        assert_eq!(rect.min, origin + egui::vec2(175.0, 46.0) * 2.0);
        assert_eq!(rect.height(), 38.0);
        assert_eq!(text_pos, rect.min + egui::vec2(6.0, 4.0));
        assert_eq!(text, "Raw Materials Monitor");
        let border = shapes.iter().any(|shape| {
            matches!(shape,
            egui::Shape::Rect(r) if r.stroke.color == BORDER && r.stroke.width == 2.0
                && r.rect == rect.shrink(1.0))
        });
        assert!(border, "no border");
        // Wider text, wider tip: the box is the text's advance plus five.
        let text_width = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) => Some(text.galley.size().x),
                _ => None,
            })
            .unwrap();
        assert_eq!(rect.width(), (text_width / 2.0 + 5.0).ceil() * 2.0);
        let size = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text) => Some(text.galley.job.sections[0].format.font_id.size),
            _ => None,
        });
        assert_eq!(size, Some(FONT_SIZE * 2.0));
        // Not before half a second.
        let early = frames(
            &egui::Context::default(),
            pointer,
            &[0.0, 0.1, 0.5],
            origin,
            2.0,
            |ctx| {
                tool(ctx, egui::Id::new("raw"), "Raw Materials Monitor");
            },
        );
        assert_eq!(tip_shapes(&early), None);
    }

    #[test]
    fn a_moving_pointer_keeps_the_tip_waiting() {
        let ctx = egui::Context::default();
        let origin = egui::pos2(0.0, 0.0);
        let mut shapes = Vec::new();
        for (index, time) in [0.0, 0.1, 0.4, 0.7, 1.0].into_iter().enumerate() {
            let input = egui::RawInput {
                time: Some(time),
                events: vec![egui::Event::PointerMoved(egui::pos2(
                    100.0 + index as f32,
                    100.0,
                ))],
                ..Default::default()
            };
            shapes = ctx
                .run(input, |ctx| {
                    paint(ctx, origin, 1.0);
                    tool(ctx, egui::Id::new("raw"), "Raw");
                })
                .shapes
                .into_iter()
                .map(|clipped| clipped.shape)
                .collect();
        }
        assert_eq!(tip_shapes(&shapes), None);
    }

    #[test]
    fn a_chrome_control_tips_only_when_enabled_hovered_and_uncovered() {
        let origin = egui::Pos2::ZERO;
        let control = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(60.0, 20.0));
        let run = |pointer: egui::Pos2, enabled: bool, covered: bool| {
            let ctx = egui::Context::default();
            let shapes = frames(
                &ctx,
                pointer,
                &[0.0, 0.1, 0.2, 0.3, 0.9],
                origin,
                1.0,
                |ctx| {
                    if covered {
                        egui::Area::new(egui::Id::new("cover"))
                            .fixed_pos(egui::pos2(90.0, 90.0))
                            .show(ctx, |ui| {
                                ui.allocate_exact_size(
                                    egui::vec2(100.0, 50.0),
                                    egui::Sense::hover(),
                                );
                            });
                    }
                    chrome_tool(ctx, egui::Id::new("chrome"), control, "Chrome", enabled);
                },
            );
            tip_shapes(&shapes).map(|(_, _, text)| text)
        };
        assert_eq!(
            run(egui::pos2(110.0, 105.0), true, false).as_deref(),
            Some("Chrome")
        );
        assert_eq!(run(egui::pos2(110.0, 105.0), false, false), None);
        assert_eq!(run(egui::pos2(110.0, 125.0), true, false), None);
        assert_eq!(run(egui::pos2(50.0, 105.0), true, false), None);
        assert_eq!(run(egui::pos2(110.0, 105.0), true, true), None);
        // PtInRect: the top left is inside, the right and bottom edges not.
        assert_eq!(
            run(egui::pos2(100.0, 100.0), true, false).as_deref(),
            Some("Chrome")
        );
        assert_eq!(run(egui::pos2(160.0, 105.0), true, false), None);
        assert_eq!(run(egui::pos2(110.0, 120.0), true, false), None);
    }

    #[test]
    fn a_window_control_tips_only_when_enabled_and_on_top() {
        // A disabled window shows no tip (FUN_00464890 disables the chosen
        // Personnel view tab: A0 tooltip/62, 65); nor does one under
        // another window.
        set_strings(
            TEST_STRINGS
                .iter()
                .map(|(id, text)| (*id, (*text).to_owned()))
                .collect(),
        );
        let run = |pointer: egui::Pos2, enabled: bool, ui_enabled: bool, covered: bool| {
            let shapes = frames(
                &egui::Context::default(),
                pointer,
                &[0.0, 0.1, 0.2, 0.3, 0.9],
                egui::Pos2::ZERO,
                1.0,
                |ctx| {
                    egui::Area::new(egui::Id::new("window"))
                        .fixed_pos(egui::pos2(0.0, 0.0))
                        .show(ctx, |ui| {
                            if !ui_enabled {
                                ui.disable();
                            }
                            ui.allocate_exact_size(egui::vec2(300.0, 300.0), egui::Sense::hover());
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(100.0, 100.0),
                                egui::vec2(40.0, 30.0),
                            );
                            window_tool(ui, rect, "close", 0x1501, enabled);
                        });
                    if covered {
                        egui::Area::new(egui::Id::new("over"))
                            .order(egui::Order::Foreground)
                            .fixed_pos(egui::pos2(90.0, 90.0))
                            .show(ctx, |ui| {
                                ui.allocate_exact_size(
                                    egui::vec2(100.0, 100.0),
                                    egui::Sense::hover(),
                                );
                            });
                    }
                },
            );
            tip_shapes(&shapes).map(|(_, _, text)| text)
        };
        let inside = egui::pos2(110.0, 110.0);
        assert_eq!(
            run(inside, true, true, false).as_deref(),
            Some("System Finder")
        );
        assert_eq!(
            run(egui::pos2(100.0, 100.0), true, true, false).as_deref(),
            Some("System Finder")
        );
        assert_eq!(run(egui::pos2(140.0, 110.0), true, true, false), None);
        assert_eq!(run(egui::pos2(110.0, 130.0), true, true, false), None);
        assert_eq!(run(inside, false, true, false), None);
        assert_eq!(run(inside, true, false, false), None);
        assert_eq!(run(inside, true, true, true), None);
    }

    #[test]
    fn a_hovered_response_tips_inside_its_rectangle() {
        set_strings(
            TEST_STRINGS
                .iter()
                .map(|(id, text)| (*id, (*text).to_owned()))
                .collect(),
        );
        let run = |pointer: egui::Pos2| {
            let shapes = frames(
                &egui::Context::default(),
                pointer,
                &[0.0, 0.1, 0.2, 0.3, 0.9],
                egui::Pos2::ZERO,
                1.0,
                |ctx| {
                    egui::Area::new(egui::Id::new("buttons"))
                        .fixed_pos(egui::pos2(0.0, 0.0))
                        .show(ctx, |ui| {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(20.0, 20.0),
                                egui::vec2(40.0, 40.0),
                            );
                            let response =
                                ui.interact(rect, ui.id().with("b"), egui::Sense::click());
                            response_tool(&response, rect, 0x1501);
                        });
                },
            );
            tip_shapes(&shapes).map(|(_, _, text)| text)
        };
        assert_eq!(
            run(egui::pos2(30.0, 30.0)).as_deref(),
            Some("System Finder")
        );
        // egui's response covers the right edge; PtInRect does not.
        assert_eq!(run(egui::pos2(60.0, 30.0)), None);
        assert_eq!(run(egui::pos2(200.0, 200.0)), None);
    }

    #[test]
    fn a_hovered_response_registers_its_tip() {
        let ctx = egui::Context::default();
        let shapes = frames(
            &ctx,
            egui::pos2(30.0, 30.0),
            &[0.0, 0.1, 0.2, 0.3, 0.9],
            egui::Pos2::ZERO,
            1.0,
            |ctx| {
                egui::Area::new(egui::Id::new("button"))
                    .fixed_pos(egui::pos2(20.0, 20.0))
                    .show(ctx, |ui| {
                        let (_, response) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
                        attach(&response, "Close");
                    });
            },
        );
        assert_eq!(
            tip_shapes(&shapes).map(|(_, _, text)| text).as_deref(),
            Some("Close")
        );
        let away = frames(
            &egui::Context::default(),
            egui::pos2(300.0, 300.0),
            &[0.0, 0.1, 0.2, 0.3, 0.9],
            egui::Pos2::ZERO,
            1.0,
            |ctx| {
                egui::Area::new(egui::Id::new("button"))
                    .fixed_pos(egui::pos2(20.0, 20.0))
                    .show(ctx, |ui| {
                        let (_, response) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
                        attach(&response, "Close");
                    });
            },
        );
        assert_eq!(tip_shapes(&away), None);
    }

    #[test]
    fn the_tip_sits_below_the_pointer_and_inside_the_client() {
        // A0 tooltip/01.png: pointer (175, 25), "Raw Materials Monitor"
        // (Arial-metric 119.4 wide at em 12) in a 124 by 19 tip at (175, 46).
        assert_eq!(
            tip_rect((175.0, 25.0), 119.4),
            egui::Rect::from_min_size(egui::pos2(175.0, 46.0), egui::vec2(125.0, 19.0))
        );
        // Off the right edge it moves left; off the bottom it goes above.
        let right = tip_rect((600.0, 25.0), 119.4);
        assert_eq!((right.min.x, right.max.x), (514.0, 639.0));
        let low = tip_rect((150.0, 440.0), 75.0);
        assert_eq!((low.min.y, low.max.y), (421.0, 440.0));
        // One that would end at 640 ends at 639 instead.
        let edge = tip_rect((515.0, 25.0), 120.0);
        assert_eq!((edge.min.x, edge.max.x), (514.0, 639.0));
        // A tip that just fits stays where it is.
        let fits = tip_rect((514.0, 439.0), 120.0);
        assert_eq!((fits.min.x, fits.min.y), (514.0, 460.0));
    }
}
