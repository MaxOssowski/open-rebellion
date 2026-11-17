//! The Finders' name box: a `CoolStringField` (`FUN_00604cf0`), the game's
//! own edit control, which the four Finders create at (143, 45), 250 wide
//! (`FUN_004637a0`, `FUN_00461960`, `FUN_0046ce40`, `FUN_00460270`).
//!
//! - **Font.** None of the Finders sets one (`vtable+0x18`, `FUN_00604f70`),
//!   so the field keeps what its base window took in `FUN_005ff910`: the
//!   game's default font (`FUN_00603850`), font 4, Arial at 13 ppem.
//! - **Text.** `WM_PAINT` (`Class_0066e038_vt14`, `FUN_00605160`) draws it
//!   with `DrawTextA` format `0x824` (single line, vertically centred, left)
//!   in (0, 0, width - 1, height - 1) through `FUN_00600fd0`: first in
//!   `+0xe8` (black, the constructor's 0) one pixel (`+0xe0`) down and
//!   right, then in `+0xe4` (white). The Finders set `+0xe4` to `0xffffff`.
//! - **Selection.** With the focus and a selection, the selected letters
//!   are drawn opaque in `+0xe8` on `+0xe4` (black on white), no shadow;
//!   the letters before and after are drawn as above from their own lefts.
//! - **Caret.** `WM_SETFOCUS` (7) creates a Windows caret 2 wide and as high
//!   as the field (`CreateCaret(hwnd, 0, 2, +0x3c)`); `FUN_006060f0` puts it
//!   at the width of the letters before it, less the scroll `+0xd4`, and
//!   scrolls the text so it stays inside.
//! - **Setting the text.** `FUN_00604f90` sets the caret index `+0xcc` to
//!   -1, which `FUN_006060f0` reads as the text's end.
//!
//! The keys, the pointer, selection and the clipboard stay with egui's
//! `TextEdit`, drawn invisibly over the field; this module paints what the
//! field shows.

use std::ops::Range;

use egui_macroquad::egui::{self, Color32};

use crate::theme::{GdiAlign, GdiVAlign};

/// The field's font: the game's default (`FUN_005ff910` → `FUN_00603850`).
pub(crate) const FONT: u16 = crate::DEFAULT_FONT;

/// The text's colour, `+0xe4` (`0xffffff`, set by each Finder).
const TEXT: Color32 = Color32::WHITE;

/// The shadow's colour, `+0xe8` (0, the constructor's `param_1[0x3a]`).
const SHADOW: Color32 = Color32::BLACK;

/// The shadow's offset, `+0xe0` (1, the constructor's `param_1[0x38]`).
const SHADOW_OFFSET: f32 = 1.0;

/// The caret's width (`CreateCaret(.., 2, ..)` in `Class_0066e038_vt14`).
const CARET_WIDTH: f32 = 2.0;

/// The Windows caret's blink period in seconds (the system default,
/// `GetCaretBlinkTime`, 530 ms). port: the original's caret blinks at the
/// system's setting; the port takes the default.
const CARET_BLINK: f64 = 0.53;

/// The caret's colour over the field's blue. port: Windows inverts the
/// pixels under the caret (an XOR in the game's 8-bit palette); A0
/// finder-search/01-open shows (103, 100, 88) where the box's blue
/// (30, 62, 166) lies under it, which the port paints over the whole caret.
const CARET: Color32 = Color32::from_rgb(103, 100, 88);

/// The line's top below the field's: `DT_VCENTER` centres the font's
/// `tmHeight` in the rectangle (0, 0, width - 1, height - 1), halving the
/// spare rows with C's division, which rounds toward zero
/// (`FUN_00605160`, format `0x824`).
#[must_use]
pub(crate) fn line_top(height: f32) -> f32 {
    ((height - SHADOW_OFFSET - crate::theme::line_pitch(FONT)) / 2.0).trunc()
}

/// The width of the text's first `chars` letters, as `DrawTextA` measures
/// them (`FUN_006060f0`, format `0xc24`), by `measure`.
#[must_use]
pub(crate) fn prefix_width(text: &str, chars: usize, measure: &dyn Fn(&str) -> f32) -> f32 {
    let end = text
        .char_indices()
        .nth(chars)
        .map_or(text.len(), |(at, _)| at);
    if end == 0 {
        0.0
    } else {
        measure(&text[..end])
    }
}

/// The scroll `+0xd4` after the caret moves to `caret_x` in a field
/// `width` wide (`FUN_006060f0`): past the right edge (`width - +0xe0`)
/// the text scrolls so the caret sits there; left of the field's left it
/// scrolls so the caret sits at it; otherwise it stays.
#[must_use]
pub(crate) fn scroll_to_caret(scroll: f32, caret_x: f32, width: f32) -> f32 {
    let at = caret_x - scroll;
    if width - SHADOW_OFFSET < at {
        caret_x + SHADOW_OFFSET - width
    } else if at < 0.0 {
        caret_x
    } else {
        scroll
    }
}

/// One piece of the text `WM_PAINT` draws with one `DrawTextA`: its
/// letters, its left before the scroll, and whether it is the selection.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Run {
    pub(crate) chars: Range<usize>,
    pub(crate) left: f32,
    pub(crate) selected: bool,
}

/// The pieces `WM_PAINT` (`FUN_00605160`) draws `text` in: the whole text
/// from the left without the focus or with an empty selection; else the
/// letters before the selection (if any), the selection from their width
/// (`+0xbc`), and the letters after it (if any) from the width of the text
/// to the selection's end (`+0xc0`).
#[must_use]
pub(crate) fn runs(
    text: &str,
    selection: Option<(usize, usize)>,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Run> {
    let count = text.chars().count();
    let Some((start, end)) = selection
        .map(|(a, b)| (a.min(b).min(count), a.max(b).min(count)))
        .filter(|(start, end)| start != end)
    else {
        return vec![Run {
            chars: 0..count,
            left: 0.0,
            selected: false,
        }];
    };
    let mut runs = Vec::with_capacity(3);
    if start > 0 {
        runs.push(Run {
            chars: 0..start,
            left: 0.0,
            selected: false,
        });
    }
    runs.push(Run {
        chars: start..end,
        left: prefix_width(text, start, measure),
        selected: true,
    });
    if end < count {
        runs.push(Run {
            chars: end..count,
            left: prefix_width(text, end, measure),
            selected: false,
        });
    }
    runs
}

fn letters(text: &str, chars: &Range<usize>) -> String {
    text.chars().skip(chars.start).take(chars.len()).collect()
}

/// Show the name box at the screen rectangle `rect` (the field's logical
/// size times `scale`), editing `text`. Returns the edit's response.
pub(crate) fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    scale: f32,
    id: egui::Id,
    text: &mut String,
) -> egui::Response {
    // FUN_00604f90: text set from outside (a row's name, an emptied box)
    // puts the caret at its end.
    let seen_id = id.with("name-box-text");
    let seen = ui.data(|data| data.get_temp::<String>(seen_id));
    if seen.as_deref() != Some(text.as_str()) {
        if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
            let end = egui::text::CCursor::new(text.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ui.ctx(), id);
        }
    }

    let outer_clip = ui.clip_rect();
    let clip = rect.intersect(outer_clip);
    let output = ui
        .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_clip_rect(clip);
            let visuals = ui.visuals_mut();
            visuals.selection.bg_fill = Color32::TRANSPARENT;
            visuals.selection.stroke = egui::Stroke::NONE;
            visuals.text_cursor.stroke = egui::Stroke::NONE;
            egui::TextEdit::singleline(text)
                .id(id)
                .frame(false)
                .margin(egui::Margin::ZERO)
                .font(crate::theme::gdi_font(FONT, scale))
                .text_color(Color32::TRANSPARENT)
                .desired_width(rect.width())
                .min_size(rect.size())
                .show(ui)
        })
        .inner;
    ui.data_mut(|data| data.insert_temp(seen_id, text.clone()));
    paint(ui, clip, (rect, scale), id, text, &output);
    output.response
}

/// Paint what the field shows (`WM_PAINT`, `FUN_00605160`) and its caret
/// (`FUN_006060f0`) over the invisible edit's `output`.
fn paint(
    ui: &egui::Ui,
    clip: egui::Rect,
    (rect, scale): (egui::Rect, f32),
    id: egui::Id,
    text: &str,
    output: &egui::text_edit::TextEditOutput,
) {
    let painter = ui.painter().with_clip_rect(clip);
    let measure = |letters: &str| crate::theme::gdi_text_width(&painter, FONT, letters, scale);
    let focused = output.response.has_focus();
    let count = text.chars().count();
    let (caret, selection) =
        output
            .cursor_range
            .filter(|_| focused)
            .map_or((count, None), |range| {
                let primary = range.primary.ccursor.index.min(count);
                let secondary = range.secondary.ccursor.index.min(count);
                (primary, Some((primary, secondary)))
            });
    let caret_x = prefix_width(text, caret, &measure);
    let scroll_id = id.with("name-box-scroll");
    let scroll = scroll_to_caret(
        ui.data(|data| data.get_temp::<f32>(scroll_id))
            .unwrap_or(0.0),
        caret_x,
        rect.width() / scale,
    );
    ui.data_mut(|data| data.insert_temp(scroll_id, scroll));

    let logical = |x: f32, y: f32, w: f32, h: f32| {
        egui::Rect::from_min_size(
            rect.min + egui::vec2(x * scale, y * scale),
            egui::vec2(w * scale, h * scale),
        )
    };
    let top = line_top(rect.height() / scale);
    let pitch = crate::theme::line_pitch(FONT);
    let format = (GdiAlign::Left, GdiVAlign::Top);
    for run in runs(text, selection.filter(|_| focused), &measure) {
        let letters = letters(text, &run.chars);
        let left = run.left - scroll;
        if run.selected {
            let width = measure(&letters);
            painter.rect_filled(logical(left, top, width, pitch), 0.0, TEXT);
            crate::theme::paint_gdi_text(
                &painter,
                logical(left, top, width, pitch),
                scale,
                FONT,
                &letters,
                SHADOW,
                format,
            );
            continue;
        }
        for (offset, colour) in [(SHADOW_OFFSET, SHADOW), (0.0, TEXT)] {
            crate::theme::paint_gdi_text(
                &painter,
                logical(left + offset, top + offset, 1.0e4, pitch),
                scale,
                FONT,
                &letters,
                colour,
                format,
            );
        }
    }

    if focused {
        let now = ui.input(|input| input.time);
        let phase = now % (2.0 * CARET_BLINK);
        if phase < CARET_BLINK {
            painter.rect_filled(
                logical(caret_x - scroll, 0.0, CARET_WIDTH, rect.height() / scale),
                0.0,
                CARET,
            );
        }
        #[expect(clippy::cast_possible_truncation, reason = "Under a second.")]
        let wait = (CARET_BLINK - phase % CARET_BLINK) as f32;
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f32(wait.max(0.01)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GDI's width at font 4's 13 ppem (Arial's hdmx and kern pairs).
    fn gdi(letters: &str) -> f32 {
        crate::theme::gdi_pen_positions(FONT, letters).unwrap().1
    }

    #[test]
    fn the_line_starts_at_the_top_of_both_the_18_and_the_16_row_boxes() {
        // FUN_00605160: DrawTextA 0x824 (DT_VCENTER) in (0, 0, w - 1,
        // h - 1) with font 4's 16-row line; C's division puts the line's
        // top at 0 in the Personnel, Troop and System Finders' 18-row box
        // and in the Fleet Finder's 16-row one (FUN_00461960). A0 d-02
        // (Personnel) and personnel-states/09 (Fleet) both set the caps
        // at y 98, three rows under the box's top at 95.
        assert_eq!(line_top(18.0), 0.0);
        assert_eq!(line_top(16.0), 0.0);
        // The same rule elsewhere: a spare row more each side moves the
        // line down one; a box shorter than the line moves it up, toward
        // zero.
        assert_eq!(line_top(20.0), 1.0);
        assert_eq!(line_top(14.0), -1.0);
    }

    #[test]
    fn the_caret_stands_at_the_gdi_width_of_the_letters_before_it() {
        // FUN_006060f0: DrawTextA(text, +0xcc letters, 0xc24) measures the
        // caret's x.
        assert_eq!(prefix_width("Luke Skywalker", 0, &gdi), 0.0);
        assert_eq!(prefix_width("Luke Skywalker", 4, &gdi), gdi("Luke"));
        assert_eq!(
            prefix_width("Luke Skywalker", 99, &gdi),
            gdi("Luke Skywalker")
        );
    }

    #[test]
    fn a_caret_past_either_edge_scrolls_the_text_to_keep_it_inside() {
        // FUN_006060f0: past width - +0xe0 the scroll becomes caret + 1 -
        // width; left of 0 it becomes the caret; between, it stays.
        assert_eq!(scroll_to_caret(0.0, 100.0, 250.0), 0.0);
        assert_eq!(scroll_to_caret(0.0, 249.0, 250.0), 0.0);
        assert_eq!(scroll_to_caret(0.0, 250.0, 250.0), 1.0);
        assert_eq!(scroll_to_caret(0.0, 260.0, 250.0), 11.0);
        assert_eq!(scroll_to_caret(11.0, 120.0, 250.0), 11.0);
        assert_eq!(scroll_to_caret(11.0, 5.0, 250.0), 5.0);
    }

    /// Run one frame of a name box holding `text`, focused, with `events`.
    fn frame(ctx: &egui::Context, text: &mut String, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 100.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(250.0, 18.0));
                let response = show(ui, rect, 1.0, egui::Id::new("test-name-box"), text);
                response.request_focus();
            });
        });
    }

    #[test]
    fn a_name_set_from_outside_puts_the_caret_at_its_end() {
        // FUN_00604f90 (a chosen row's name, FUN_00464890 case 0xca
        // 0x29b) sets the caret index +0xcc to -1, which FUN_006060f0
        // reads as the text's end: a letter typed next follows the name.
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut text = String::new();
        frame(&ctx, &mut text, vec![]);
        frame(&ctx, &mut text, vec![egui::Event::Text("Fl".into())]);
        assert_eq!(text, "Fl");
        text = "Fleet 1".into();
        frame(&ctx, &mut text, vec![]);
        frame(&ctx, &mut text, vec![egui::Event::Text("x".into())]);
        assert_eq!(text, "Fleet 1x");
        // Text the box itself changed keeps its caret: Home, then a letter
        // in the next frame goes first.
        let home = egui::Event::Key {
            key: egui::Key::Home,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&ctx, &mut text, vec![home]);
        frame(&ctx, &mut text, vec![egui::Event::Text("y".into())]);
        assert_eq!(text, "yFleet 1x");
    }

    #[test]
    fn a_run_paints_only_its_own_letters() {
        // FUN_00605160 passes each DrawTextA its run's letters alone.
        assert_eq!(letters("Fleet 6", &(2..4)), "ee");
        assert_eq!(letters("Fleet 6", &(4..7)), "t 6");
    }

    #[test]
    fn a_selection_is_drawn_apart_between_the_letters_before_and_after_it() {
        // FUN_00605160: without a selection the whole text from the left;
        // with one, the letters before, the selection from +0xbc and the
        // rest from +0xc0, each its own DrawTextA.
        let plain = |chars: Range<usize>, left: f32| Run {
            chars,
            left,
            selected: false,
        };
        assert_eq!(runs("Fleet 6", None, &gdi), vec![plain(0..7, 0.0)]);
        assert_eq!(runs("Fleet 6", Some((3, 3)), &gdi), vec![plain(0..7, 0.0)]);
        assert_eq!(
            runs("Fleet 6", Some((4, 2)), &gdi),
            vec![
                plain(0..2, 0.0),
                Run {
                    chars: 2..4,
                    left: gdi("Fl"),
                    selected: true
                },
                plain(4..7, gdi("Flee")),
            ]
        );
        assert_eq!(
            runs("Fleet 6", Some((0, 7)), &gdi),
            vec![Run {
                chars: 0..7,
                left: 0.0,
                selected: true
            }]
        );
    }
}
