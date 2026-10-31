//! Star Wars Rebellion egui theme.
//!
//! Dark space background with gold/amber accents, faction-colored elements,
//! and Liberation Sans font (metrically identical to the original game's Arial).
//!
//! Architecture: macroquad renders BMP backgrounds first, then egui panels
//! overlay on top. Colors here match the original game palette so that when
//! DLL-extracted cockpit frames and button sprites are loaded, they integrate
//! seamlessly with the themed UI.

use egui_macroquad::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Style, Visuals};

// ── Color palette ────────────────────────────────────────────────────────────

/// Deep space background — nearly black with a hint of blue.
pub const BG_SPACE: Color32 = Color32::from_rgb(5, 5, 16);
/// Slightly lighter panel background.
pub const BG_PANEL: Color32 = Color32::from_rgb(12, 14, 24);
/// Panel/window frame color.
pub const BG_FRAME: Color32 = Color32::from_rgb(22, 24, 36);
/// Gold accent — button highlights, selection, headers.
pub const GOLD: Color32 = Color32::from_rgb(218, 165, 32);
/// Darker gold for hover states.
pub const GOLD_DIM: Color32 = Color32::from_rgb(160, 120, 24);
/// Bright gold for active/pressed states.
pub const GOLD_BRIGHT: Color32 = Color32::from_rgb(255, 200, 60);
/// Alliance blue.
pub const ALLIANCE_BLUE: Color32 = Color32::from_rgb(100, 160, 255);
/// Empire red.
pub const EMPIRE_RED: Color32 = Color32::from_rgb(220, 80, 80);
/// Neutral / unaligned gray.
pub const NEUTRAL_GRAY: Color32 = Color32::from_rgb(140, 140, 140);
/// Primary text color — off-white.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 215, 200);
/// Secondary text color — muted.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 145, 135);
/// Disabled text.
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(80, 78, 72);
/// Success / positive indicator.
pub const SUCCESS_GREEN: Color32 = Color32::from_rgb(60, 180, 80);
/// Warning / caution.
pub const WARNING_AMBER: Color32 = Color32::from_rgb(220, 160, 40);
/// Error / danger.
pub const DANGER_RED: Color32 = Color32::from_rgb(200, 50, 50);

// ── Font loading ─────────────────────────────────────────────────────────────

/// Liberation Sans 2.1.5 (SIL OFL 1.1, `assets/fonts/LICENSE-OFL.txt`), the
/// metric match of Arial: the original builds every font of its table on
/// the face "Arial" (`FUN_0060ee20` copies `0x6ac7b8` to the table's `+0x38`,
/// `FUN_0060eed0` creates each font from it). Built in, so the browser and
/// any working directory get it.
pub(crate) const ARIAL_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf");
pub(crate) const ARIAL_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/LiberationSans-Bold.ttf");

/// The bold face's family name in egui.
pub const ARIAL_BOLD_FAMILY: &str = "arial-bold";

/// Load the Arial-metric faces into egui: the regular one first for the
/// proportional family, the bold one as [`ARIAL_BOLD_FAMILY`].
///
/// Call once at startup.
pub fn load_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "liberation-sans".to_owned(),
        std::sync::Arc::new(FontData::from_static(ARIAL_REGULAR)),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "liberation-sans".to_owned());
    fonts.font_data.insert(
        "liberation-sans-bold".to_owned(),
        std::sync::Arc::new(FontData::from_static(ARIAL_BOLD)),
    );
    let mut bold = vec!["liberation-sans-bold".to_owned()];
    bold.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .into_iter()
            .flatten()
            .cloned(),
    );
    fonts
        .families
        .insert(FontFamily::Name(ARIAL_BOLD_FAMILY.into()), bold);
    fonts
}

/// One font of the original's table (`FUN_0060eed0`), by index: its cell
/// height in pixels (`lfHeight`, positive, so ascent plus descent) and
/// whether it is bold (weight 700 or 900).
///
/// | Index | Cell | Weight |
/// |---|---|---|
/// | 1, 5 | 16 | 700 |
/// | 2 | 14 | 700 |
/// | 3 | 8 | 400 |
/// | 6 | 18 | 400 |
/// | 7 | 18 | 700 |
/// | 8, 9 | 12 | 400 |
/// | 10 | 14 | 400 |
/// | 11 | 24 | 400 |
/// | 12 | 30 | 400 |
/// | 13 | 12 | 900 |
/// | others | 16 | 400 |
#[must_use]
pub fn original_font_cell(font: u16) -> (f32, bool) {
    match font {
        1 | 5 => (16.0, true),
        2 => (14.0, true),
        3 => (8.0, false),
        6 => (18.0, false),
        7 => (18.0, true),
        8 | 9 => (12.0, false),
        10 => (14.0, false),
        11 => (24.0, false),
        12 => (30.0, false),
        13 => (12.0, true),
        _ => (16.0, false),
    }
}

/// Arial's em (`unitsPerEm`) and its cell, ascent plus descent (`usWinAscent`
/// 1854 + `usWinDescent` 434); Liberation Sans shares both.
const ARIAL_UNITS_PER_EM: f32 = 2048.0;
const ARIAL_CELL_UNITS: f32 = 1854.0 + 434.0;

/// The pixels per em GDI draws font `font` at. For a positive `lfHeight`
/// GDI takes the tallest hinted cell of Arial's VDMX table (`yMax - yMin`)
/// that fits the height, at the smallest ppem that has it: the 14-pixel
/// cell is 11 ppem (VDMX 11 + 3), the 16-pixel one 13 (13 + 3, as 14 is).
/// Read from Windows 10's arial.ttf and arialbd.ttf (VDMX ratio 1:1); only
/// these numbers are kept (ghidra/notes/gdi-font-sizes.md). A0 confirms 11
/// (sector-windows/04-left, planet names in font 10), 13 (the sector title
/// in font 4) and bold 13 (the pause box in font 5, tooltip/321): drawn at
/// those sizes, the names and the title match A0 pixel for pixel.
/// [`original_font`] still sizes egui by the cell; moving it to these
/// sizes moves every text, window by window against A0.
#[must_use]
pub fn original_font_ppem(font: u16) -> f32 {
    match font {
        1 | 5 => 13.0,
        2 => 11.0,
        3 => 7.0,
        6 => 16.0,
        7 => 15.0,
        8 | 9 => 9.0,
        10 => 11.0,
        11 => 21.0,
        12 => 25.0,
        13 => 10.0,
        _ => 13.0,
    }
}

/// The egui font for the original's font `font` at `scale`. GDI's positive
/// `lfHeight` is the cell, ascent plus descent; egui's size is the em
/// (epaint scales it by the face's height over its em), so the cell
/// becomes `cell x 2048 / 2288`: 14.3 for the 16-pixel cell.
#[must_use]
pub fn original_font(font: u16, scale: f32) -> egui::FontId {
    let (cell, bold) = original_font_cell(font);
    let em = cell * ARIAL_UNITS_PER_EM / ARIAL_CELL_UNITS;
    let family = if bold {
        FontFamily::Name(ARIAL_BOLD_FAMILY.into())
    } else {
        FontFamily::Proportional
    };
    egui::FontId::new((em * scale).max(1.0), family)
}

// ── Theme application ────────────────────────────────────────────────────────

/// Apply the Star Wars Rebellion theme to the egui context.
///
/// Sets dark space visuals with gold/amber accents. Call once at startup.
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = Style::default();

    // ── Visuals ──────────────────────────────────────────────────────────────
    let mut visuals = Visuals::dark();

    // Window / panel backgrounds
    visuals.window_fill = BG_PANEL;
    visuals.panel_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_SPACE;
    visuals.faint_bg_color = BG_FRAME;

    // Window frame / border
    visuals.window_stroke = egui::Stroke::new(1.0_f32, GOLD_DIM);

    // Selection color
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(218, 165, 32, 60);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, GOLD);

    // Hyperlink color
    visuals.hyperlink_color = GOLD;

    // Widget styles — inactive
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 32, 48);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(25, 27, 40);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(50, 52, 68));
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(3);

    // Widget styles — hovered
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 42, 60);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(35, 37, 52);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, GOLD_DIM);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, GOLD);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(3);

    // Widget styles — active (pressed)
    visuals.widgets.active.bg_fill = Color32::from_rgb(50, 48, 30);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(45, 43, 28);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5_f32, GOLD_BRIGHT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5_f32, GOLD_BRIGHT);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(3);

    // Widget styles — open (expanded combo boxes, etc.)
    visuals.widgets.open.bg_fill = Color32::from_rgb(35, 37, 52);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(30, 32, 46);
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, GOLD_DIM);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, GOLD);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(3);

    // Widget styles — non-interactive (labels, etc.)
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(0.5_f32, Color32::from_rgb(40, 42, 56));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(2);

    // Window and menu rounding
    visuals.window_corner_radius = egui::CornerRadius::same(4);
    visuals.menu_corner_radius = egui::CornerRadius::same(4);

    style.visuals = visuals;

    // ── Spacing ──────────────────────────────────────────────────────────────
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);

    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_fonts_take_their_cells_and_weights() {
        // FUN_0060eed0's switch on the font index: lfHeight and lfWeight.
        let cells: Vec<(f32, bool)> = (0..=14).map(original_font_cell).collect();
        assert_eq!(
            cells,
            [
                (16.0, false),
                (16.0, true),
                (14.0, true),
                (8.0, false),
                (16.0, false),
                (16.0, true),
                (18.0, false),
                (18.0, true),
                (12.0, false),
                (12.0, false),
                (14.0, false),
                (24.0, false),
                (30.0, false),
                (12.0, true),
                (16.0, false),
            ]
        );
        // egui takes the em: the cell x 2048 / 2288, scaled; bold takes the
        // bold face.
        let regular = original_font(10, 2.0);
        assert_eq!(regular.family, FontFamily::Proportional);
        assert!((regular.size - 28.0 * 2048.0 / 2288.0).abs() < 1e-4);
        assert!((original_font(4, 1.0).size - 14.3217).abs() < 1e-3);
        let bold = original_font(7, 1.0);
        assert_eq!(bold.family, FontFamily::Name(ARIAL_BOLD_FAMILY.into()));
        assert!((bold.size - 18.0 * 2048.0 / 2288.0).abs() < 1e-4);
    }

    #[test]
    fn gdi_draws_each_font_at_the_ppem_arials_vdmx_fits_in_its_cell() {
        // Windows 10 arial.ttf / arialbd.ttf VDMX: the tallest yMax - yMin
        // within lfHeight, at its smallest ppem. A0 sector-windows/04-left
        // (font 10: 11) and its title (font 4: 13), tooltip/321 (font 5:
        // bold 13).
        let ppems: Vec<f32> = (0..=14).map(original_font_ppem).collect();
        assert_eq!(
            ppems,
            [
                13.0, 13.0, 11.0, 7.0, 13.0, 13.0, 16.0, 15.0, 9.0, 9.0, 11.0, 21.0, 25.0, 10.0,
                13.0
            ]
        );
        // Each is below the cell's share of the em (cell x 2048 / 2288).
        for font in 0..=14 {
            let (cell, _) = original_font_cell(font);
            assert!(original_font_ppem(font) < cell * 2048.0 / 2288.0);
        }
    }

    #[test]
    fn the_arial_metric_faces_lead_their_families() {
        let fonts = font_definitions();
        assert_eq!(
            fonts.families[&FontFamily::Proportional]
                .first()
                .map(String::as_str),
            Some("liberation-sans")
        );
        assert_eq!(
            fonts.families[&FontFamily::Name(ARIAL_BOLD_FAMILY.into())]
                .first()
                .map(String::as_str),
            Some("liberation-sans-bold")
        );
        assert!(fonts.font_data.contains_key("liberation-sans-bold"));
    }
}
