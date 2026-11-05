//! The REBDLOG question and message boxes (`FUN_00416c50`, `FUN_00417020`,
//! `FUN_00401110`): background 10623 (a question, Yes and No) or 10622 (a
//! message, one button), 412 by 176 at (114, 152) of a 640 by 480 screen,
//! the texts in font 5 and the buttons 10624..10627.
//!
//! The screen is 8-bit, so GDI shows the dialog's DIBs in the window's
//! realized palette: each colour becomes its nearest palette entry. The
//! caller names the screen's palette (COMMON 20002's in Game Options, the
//! shuttle cockpit's over the main menu).

use egui_macroquad::egui::{self, Color32, Pos2, Rect, Vec2};

use crate::bmp_cache::{BmpCache, DllSource};

/// Which box: a question with Yes and No, or a message with one button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Question,
    Message,
}

/// A box's texts: an optional first string word-wrapped in its top band,
/// and a second centred on one line (`Class_00658950_vt3c`).
pub struct Texts<'a> {
    pub first: Option<&'a str>,
    pub second: &'a str,
}

/// Where the 640 by 480 screen lies: its corner and its scale.
#[derive(Debug, Clone, Copy)]
pub struct Screen {
    pub origin: Pos2,
    pub scale: f32,
}

impl Screen {
    fn rect(self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            self.origin + Vec2::new(x, y) * self.scale,
            Vec2::new(w, h) * self.scale,
        )
    }
}

/// Paint the box over `screen` in the realized `palette` (named `name` for
/// the cache; empty for the DIBs' own colours) and return the answer:
/// `Some(true)` for Yes (or the message's button, or Enter), `Some(false)`
/// for No. Missing original assets fall back to plain buttons.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Keep the box's ordered paint and input pass together, as Game Options had it."
)]
pub fn paint(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    screen: Screen,
    kind: Kind,
    texts: &Texts<'_>,
    palette: &[[u8; 3]],
    name: &'static str,
) -> Option<bool> {
    let mut answer = None;
    let background = match kind {
        Kind::Question => 10623,
        Kind::Message => 10622,
    };
    let assets = [background, 10624, 10625, 10626, 10627];
    let complete = assets.iter().all(|id| {
        cache.get(ctx, DllSource::Rebdlog, *id).is_some()
            && cache
                .original_resource_size(DllSource::Rebdlog, *id)
                .is_some()
    });
    let bounds = screen.rect(114.0, 152.0, 412.0, 176.0);
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let dialog_texture = |cache: &mut BmpCache, id: u32| {
        if palette.is_empty() {
            cache
                .get(ctx, DllSource::Rebdlog, id)
                .map(|texture| texture.id())
        } else {
            cache
                .get_in_screen_palette(ctx, DllSource::Rebdlog, id, name, palette)
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
    // 0x2f0fbff, centred and word-wrapped from the top of (43, 25) to
    // (368, 86); the second centred on one line in (43, 87) to (368, 112),
    // or in (43, 25) to (368, 112) alone. DrawTextA through
    // theme::paint_gdi_text(_block).
    let cream = Color32::from_rgb(255, 251, 240);
    let (left, top) = (114.0 + 43.0, 152.0);
    if let Some(first) = texts.first {
        let _ = crate::theme::paint_gdi_text_block(
            ui.painter(),
            screen.rect(left, top + 25.0, 325.0, 61.0),
            screen.scale,
            5,
            first,
            cream,
            crate::theme::GdiAlign::Center,
        );
    }
    let (band_top, band_height) = if texts.first.is_some() {
        (87.0, 25.0)
    } else {
        (25.0, 87.0)
    };
    crate::theme::paint_gdi_text(
        ui.painter(),
        screen.rect(left, top + band_top, 325.0, band_height),
        screen.scale,
        5,
        texts.second,
        cream,
        (
            crate::theme::GdiAlign::Center,
            crate::theme::GdiVAlign::VCenter,
        ),
    );
    // FUN_00416e90: Yes shows 0x2981 and 0x2980 when held, No 0x2983 and
    // 0x2982 (A0 tooltip/211, 212 show 10625 and 10627). The message's one
    // button (Class_006589a8_vt40) has Yes's art at (290, 286).
    let buttons: &[(f32, f32, u32, u32, bool, &str)] = match kind {
        Kind::Message => &[(290.0, 286.0, 10625, 10624, true, "Yes")],
        Kind::Question => &[
            (252.0, 287.0, 10625, 10624, true, "Yes"),
            (343.0, 287.0, 10627, 10626, false, "No"),
        ],
    };
    // Class_00658950_vt4c / Class_006589a8_vt4c: Enter answers Yes.
    if ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
        answer = Some(true);
    }
    for &(x, y, normal, pressed, value, label) in buttons {
        let bounds = screen.rect(x, y, 57.0, 28.0);
        if complete {
            let response = ui.interact(bounds, ui.id().with(normal), egui::Sense::click());
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
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "A bitmap's pixel under the pointer."
            )]
            let hit_at = |pos: Pos2, cache: &mut BmpCache| {
                cache.is_resource_hit(
                    DllSource::Rebdlog,
                    normal,
                    ((pos.x - bounds.left()) / screen.scale) as usize,
                    ((pos.y - bounds.top()) / screen.scale) as usize,
                )
            };
            if ctx.input(|i| i.pointer.primary_pressed()) {
                let hit = ctx
                    .pointer_latest_pos()
                    .is_some_and(|pos| bounds.contains(pos) && hit_at(pos, cache));
                ui.data_mut(|data| data.insert_temp(capture_id, hit));
            }
            let captured = ui.data(|data| data.get_temp::<bool>(capture_id).unwrap_or(false));
            if captured
                && response.clicked()
                && response
                    .interact_pointer_pos()
                    .is_some_and(|pos| hit_at(pos, cache))
            {
                answer = Some(value);
            }
        } else {
            let response = ui.put(bounds, egui::Button::new(label).min_size(bounds.size()));
            if response.clicked() {
                answer = Some(value);
            }
        }
    }
    answer
}

/// A bitmap's palette as a screen's realized palette: its 256 entries'
/// colours, entry 254 white as the 8-bit screens show it (A0 tooltip/211,
/// 212 and main-menu-quit/q0: every dialog pixel is its nearest entry).
#[must_use]
pub fn realized_palette(cache: &BmpCache, source: DllSource, resource_id: u32) -> Vec<[u8; 3]> {
    cache
        .original_palette_rgba(source, resource_id)
        .map(|entries| {
            let mut palette: Vec<[u8; 3]> =
                entries.iter().map(|[r, g, b, _]| [*r, *g, *b]).collect();
            palette[254] = [255, 255, 255];
            palette
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_realized_palette_is_the_bitmaps_table_with_entry_254_white() {
        // GDI's realized palette of an 8-bit screen: the screen bitmap's
        // table, entry 254 white (A0 tooltip/211, main-menu-quit/q0).
        let root = std::env::temp_dir().join(format!("rebdlog-palette-{}", std::process::id()));
        let dir = root.join("common-dll/BMP");
        std::fs::create_dir_all(&dir).unwrap();
        let offset = 54 + 1024;
        let mut bmp = vec![0u8; offset + 4];
        bmp[..2].copy_from_slice(b"BM");
        bmp[10..14].copy_from_slice(&(offset as u32).to_le_bytes());
        bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&1i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&1i32.to_le_bytes());
        bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
        bmp[28..30].copy_from_slice(&8u16.to_le_bytes());
        for index in 0..256usize {
            let at = 54 + index * 4;
            bmp[at..at + 3].copy_from_slice(&[index as u8, 3, 7]);
        }
        std::fs::write(dir.join("20001.bmp"), bmp).unwrap();
        let mut cache = BmpCache::new();
        cache.set_base_path(&root);
        let palette = realized_palette(&cache, DllSource::Common, 20001);
        assert_eq!(palette.len(), 256);
        assert_eq!(palette[5], [7, 3, 5]);
        assert_eq!(palette[253], [7, 3, 253]);
        assert_eq!(palette[254], [255, 255, 255]);
        assert!(realized_palette(&cache, DllSource::Common, 1).is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
