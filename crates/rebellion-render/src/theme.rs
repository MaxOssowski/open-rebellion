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
    // Fonts 1 and 5, bold 16, share the regular 16's 13 (the default).
    match font {
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

/// GDI's `tmAscent` and `tmDescent` for font `font`: Arial's VDMX `yMax`
/// and `-yMin` at [`original_font_ppem`] (below the table, at 7 ppem, the
/// hhea values rounded). Their sum is the line's `tmHeight`.
#[must_use]
pub fn original_font_metrics(font: u16) -> (f32, f32) {
    // Fonts 1 and 5, bold 16, share the regular 16's 13 + 3 (the default).
    match font {
        2 => (11.0, 3.0),
        3 => (6.0, 1.0),
        6 => (15.0, 3.0),
        7 => (14.0, 4.0),
        8 | 9 => (9.0, 3.0),
        10 => (11.0, 3.0),
        11 => (19.0, 5.0),
        12 => (23.0, 5.0),
        13 => (10.0, 2.0),
        _ => (13.0, 3.0),
    }
}

/// The whole-pixel cell egui draws font `font` in for [`paint_gdi_text`].
/// epaint scales egui's size, the em, by the face's cell over its em and
/// rounds that cell to whole pixels, so it cannot draw GDI's ppem exactly;
/// and GDI's hinted advances (`hdmx`) are narrower than Liberation's
/// unhinted ones. Each cell is the one whose Liberation widths come nearest
/// to GDI's at [`original_font_ppem`] over 300 TEXTSTRA strings: within
/// 0.7 to 3.3 pixels a string on average, where `lfHeight` itself was up to
/// 9.6 too wide (font 10; ghidra/notes/gdi-font-sizes.md).
fn gdi_egui_cell(font: u16) -> f32 {
    // Fonts 1 and 5, bold 16, share the regular 16's 15 (the default).
    match font {
        2 => 13.0,
        3 => 8.0,
        6 => 18.0,
        7 => 17.0,
        8 | 9 => 11.0,
        10 => 12.0,
        11 => 23.0,
        12 => 28.0,
        13 => 11.0,
        _ => 15.0,
    }
}

/// The egui font [`paint_gdi_text`] draws font `font` in at `scale`.
#[must_use]
pub fn gdi_font(font: u16, scale: f32) -> egui::FontId {
    let (_, bold) = original_font_cell(font);
    let em = gdi_egui_cell(font) * ARIAL_UNITS_PER_EM / ARIAL_CELL_UNITS;
    let family = if bold {
        FontFamily::Name(ARIAL_BOLD_FAMILY.into())
    } else {
        FontFamily::Proportional
    };
    egui::FontId::new((em * scale).max(1.0), family)
}

/// `DrawTextA`'s horizontal format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GdiAlign {
    /// `DT_LEFT`.
    Left,
    /// `DT_CENTER`: the rectangle's spare width halved, in whole pixels.
    Center,
}

/// `DrawTextA`'s vertical format for one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GdiVAlign {
    /// `DT_TOP`: the line's top at the rectangle's.
    Top,
    /// `DT_VCENTER`: the rectangle's spare height over `tmHeight`, halved
    /// in whole pixels.
    VCenter,
}

/// Where `DrawTextA` puts one line of `width` logical pixels in the
/// logical rectangle (`x`, `y`, `w`, `h`) with font `font`: the pen's left
/// and the baseline (`tmAscent` below the line's top).
#[must_use]
pub fn gdi_text_origin(
    (x, y, w, h): (f32, f32, f32, f32),
    font: u16,
    width: f32,
    (align, valign): (GdiAlign, GdiVAlign),
) -> (f32, f32) {
    let (ascent, descent) = original_font_metrics(font);
    let left = match align {
        GdiAlign::Left => x,
        GdiAlign::Center => x + ((w - width.round()) / 2.0).floor(),
    };
    let top = match valign {
        GdiVAlign::Top => y,
        GdiVAlign::VCenter => y + ((h - ascent - descent) / 2.0).floor(),
    };
    (left, top + ascent)
}

/// GDI's pen positions for `text` in font `font`, in logical pixels, and
/// the line's width: each character's advance at the font's ppem (Arial's
/// hdmx) plus the `kern` pair before the next, rounded (A0: "To" in
/// "Tokmia" and "Ya" in "Yavin" close by a pixel). `None` when a character
/// is outside cp1252 or the size has no table.
#[must_use]
pub fn gdi_pen_positions(font: u16, text: &str) -> Option<(Vec<f32>, f32)> {
    let (_, bold) = original_font_cell(font);
    let ppem = original_font_ppem(font);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The game's sizes are small whole ppems."
    )]
    let size = ppem as u8;
    let (_, _, advances) = crate::gdi_metrics::ADVANCES
        .iter()
        .find(|(weight, at, _)| *weight == bold && *at == size)?;
    let codes = text.chars().map(cp1252).collect::<Option<Vec<u8>>>()?;
    let mut pens = Vec::with_capacity(codes.len());
    let mut pen = 0.0;
    for (index, &code) in codes.iter().enumerate() {
        pens.push(pen);
        pen += f32::from(*advances.get(usize::from(code).checked_sub(32)?)?);
        if let Some(&next) = codes.get(index + 1) {
            let kern = crate::gdi_metrics::KERNING
                .binary_search_by(|&(weight, left, right, _)| {
                    (weight, left, right).cmp(&(bold, code, next))
                })
                .ok()
                .map_or(0, |at| crate::gdi_metrics::KERNING[at].3);
            pen += (f32::from(kern) * ppem / ARIAL_UNITS_PER_EM).round();
        }
    }
    Some((pens, pen))
}

/// The cp1252 code of `ch`, the game's text encoding.
fn cp1252(ch: char) -> Option<u8> {
    const HIGH: [(char, u8); 27] = [
        ('€', 0x80),
        ('‚', 0x82),
        ('ƒ', 0x83),
        ('„', 0x84),
        ('…', 0x85),
        ('†', 0x86),
        ('‡', 0x87),
        ('ˆ', 0x88),
        ('‰', 0x89),
        ('Š', 0x8a),
        ('‹', 0x8b),
        ('Œ', 0x8c),
        ('Ž', 0x8e),
        ('‘', 0x91),
        ('’', 0x92),
        ('“', 0x93),
        ('”', 0x94),
        ('•', 0x95),
        ('–', 0x96),
        ('—', 0x97),
        ('˜', 0x98),
        ('™', 0x99),
        ('š', 0x9a),
        ('›', 0x9b),
        ('œ', 0x9c),
        ('ž', 0x9e),
        ('Ÿ', 0x9f),
    ];
    let code = u32::from(ch);
    if (0x20..0x7f).contains(&code) || (0xa0..=0xff).contains(&code) {
        return u8::try_from(code).ok();
    }
    HIGH.iter().find(|(c, _)| *c == ch).map(|(_, code)| *code)
}

/// Paint `text` in font `font` as `DrawTextA` places it in the screen
/// rectangle `rect` at `scale`, through [`gdi_text_origin`]: each character
/// at GDI's pen position ([`gdi_pen_positions`]) in the egui font matched
/// to GDI's size ([`gdi_font`]), or the whole line as egui lays it out when
/// GDI's advances are not known.
pub fn paint_gdi_text(
    painter: &egui::Painter,
    rect: egui::Rect,
    scale: f32,
    font: u16,
    text: &str,
    color: Color32,
    format: (GdiAlign, GdiVAlign),
) {
    let font_id = gdi_font(font, scale);
    let baseline_of = |galley: &egui::Galley| {
        galley
            .rows
            .first()
            .and_then(|row| row.glyphs.first())
            .map_or(0.0, |glyph| glyph.pos.y)
    };
    let logical = (0.0, 0.0, rect.width() / scale, rect.height() / scale);
    if let Some((pens, width)) = gdi_pen_positions(font, text) {
        let (left, baseline) = gdi_text_origin(logical, font, width, format);
        for (ch, pen) in text.chars().zip(pens) {
            let galley = painter.layout_no_wrap(ch.to_string(), font_id.clone(), color);
            let pos = egui::pos2(
                rect.min.x + (left + pen) * scale,
                rect.min.y + baseline * scale - baseline_of(&galley),
            );
            painter.galley(pos, galley, color);
        }
        return;
    }
    let galley = painter.layout_no_wrap(text.to_owned(), font_id, color);
    let (left, baseline) = gdi_text_origin(logical, font, galley.size().x / scale, format);
    let pos = egui::pos2(
        rect.min.x + left * scale,
        rect.min.y + baseline * scale - baseline_of(&galley),
    );
    painter.galley(pos, galley, color);
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
    fn draw_text_centres_in_whole_pixels_and_sets_the_baseline_by_gdi_metrics() {
        // DT_CENTER | DT_VCENTER in the sector title's rectangle (2, 2, 231,
        // 20) with font 4 (13 + 3): a 39-pixel line starts 96 in and its
        // baseline is 2 + 2 + 13 down. DT_TOP keeps the line's top.
        assert_eq!(
            gdi_text_origin(
                (2.0, 2.0, 231.0, 20.0),
                4,
                39.0,
                (GdiAlign::Center, GdiVAlign::VCenter)
            ),
            (98.0, 17.0)
        );
        assert_eq!(
            gdi_text_origin(
                (10.0, 48.0, 100.0, 16.0),
                10,
                31.4,
                (GdiAlign::Center, GdiVAlign::Top)
            ),
            (44.0, 59.0)
        );
        assert_eq!(
            gdi_text_origin(
                (10.0, 5.0, 100.0, 21.0),
                10,
                31.0,
                (GdiAlign::Left, GdiVAlign::VCenter)
            ),
            (10.0, 5.0 + 3.0 + 11.0)
        );
        // Each font's line is no taller than its cell.
        for font in 0..=14 {
            let (ascent, descent) = original_font_metrics(font);
            assert!(ascent + descent <= original_font_cell(font).0);
            assert!(gdi_egui_cell(font) <= original_font_cell(font).0);
        }
    }

    #[test]
    fn each_font_takes_gdis_line_metrics_and_its_matched_egui_cell() {
        // VDMX yMax / -yMin at each font's ppem (gdi-font-sizes.md), and the
        // egui cells matched to GDI's widths over 300 TEXTSTRA strings.
        let metrics: Vec<(f32, f32)> = (0..=14).map(original_font_metrics).collect();
        assert_eq!(
            metrics,
            [
                (13.0, 3.0),
                (13.0, 3.0),
                (11.0, 3.0),
                (6.0, 1.0),
                (13.0, 3.0),
                (13.0, 3.0),
                (15.0, 3.0),
                (14.0, 4.0),
                (9.0, 3.0),
                (9.0, 3.0),
                (11.0, 3.0),
                (19.0, 5.0),
                (23.0, 5.0),
                (10.0, 2.0),
                (13.0, 3.0),
            ]
        );
        let cells: Vec<f32> = (0..=14).map(gdi_egui_cell).collect();
        assert_eq!(
            cells,
            [
                15.0, 15.0, 13.0, 8.0, 15.0, 15.0, 18.0, 17.0, 11.0, 11.0, 12.0, 23.0, 28.0, 11.0,
                15.0
            ]
        );
        let regular = gdi_font(10, 2.0);
        assert_eq!(regular.family, FontFamily::Proportional);
        assert!((regular.size - 24.0 * 2048.0 / 2288.0).abs() < 1e-4);
        let bold = gdi_font(7, 1.0);
        assert_eq!(bold.family, FontFamily::Name(ARIAL_BOLD_FAMILY.into()));
        assert!((bold.size - 17.0 * 2048.0 / 2288.0).abs() < 1e-4);
    }

    /// The text shapes `paint` makes: (text, position, baseline).
    fn painted_text(paint: impl FnOnce(&egui::Painter)) -> Vec<(String, egui::Pos2, f32)> {
        let ctx = egui::Context::default();
        load_fonts(&ctx);
        let mut paint = Some(paint);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            if let Some(paint) = paint.take() {
                paint(&painter);
            }
        });
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.pos,
                    text.pos.y
                        + text
                            .galley
                            .rows
                            .first()
                            .and_then(|row| row.glyphs.first())
                            .map_or(0.0, |glyph| glyph.pos.y),
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn gdi_text_paints_each_character_at_its_pen_on_the_baseline() {
        // "Tokmia" in font 10, DT_CENTER | DT_TOP in a 100 by 16 box at
        // (5, 10), scale 2: 32 wide, so it starts 34 in; the baseline is
        // 11 (tmAscent) below the top.
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(200.0, 32.0));
        let shapes = painted_text(|painter| {
            paint_gdi_text(
                painter,
                rect,
                2.0,
                10,
                "Tokmia",
                Color32::WHITE,
                (GdiAlign::Center, GdiVAlign::Top),
            );
        });
        let text: String = shapes.iter().map(|(text, _, _)| text.as_str()).collect();
        assert_eq!(text, "Tokmia");
        let xs: Vec<f32> = shapes.iter().map(|(_, pos, _)| pos.x).collect();
        assert_eq!(xs, [78.0, 88.0, 100.0, 110.0, 126.0, 130.0]);
        for (_, _, baseline) in &shapes {
            assert!((baseline - (20.0 + 11.0 * 2.0)).abs() < 1e-3, "{shapes:?}");
        }
        // Outside cp1252 the line is one egui layout, centred by its own
        // width and set on the same baseline.
        let shapes = painted_text(|painter| {
            paint_gdi_text(
                painter,
                rect,
                2.0,
                10,
                "ΩΩ",
                Color32::WHITE,
                (GdiAlign::Left, GdiVAlign::Top),
            );
        });
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].1.x, 10.0);
        assert!((shapes[0].2 - 42.0).abs() < 1e-3);
        let centred = painted_text(|painter| {
            paint_gdi_text(
                painter,
                rect,
                2.0,
                10,
                "ΩΩ",
                Color32::WHITE,
                (GdiAlign::Center, GdiVAlign::VCenter),
            );
        });
        let width = std::cell::Cell::new(0.0);
        let _ = painted_text(|painter| {
            width.set(
                painter
                    .layout_no_wrap("ΩΩ".to_owned(), gdi_font(10, 2.0), Color32::WHITE)
                    .size()
                    .x,
            );
        });
        let spare = ((100.0 - (width.get() / 2.0).round()) / 2.0).floor();
        assert_eq!(centred[0].1.x, 10.0 + spare * 2.0);
        // DT_VCENTER: (16 - 14) / 2 = 1 down, then tmAscent.
        assert!((centred[0].2 - (20.0 + 12.0 * 2.0)).abs() < 1e-3);
    }

    #[test]
    fn gdi_advances_and_kerning_set_the_pen() {
        // A0 sector-windows/04-left: "Tokmia" at 11 ppem advances T 6, o 6,
        // k 5, m 8, i 2 (hdmx), and the "To" pair (-227 units) closes a
        // pixel; "Yavin" (build-selection/05) closes "Ya" (-152): the "a"
        // at 7, not at Y's advance 8.
        let (pens, width) = gdi_pen_positions(10, "Tokmia").unwrap();
        assert_eq!(pens, [0.0, 5.0, 11.0, 16.0, 24.0, 26.0]);
        assert_eq!(width, 32.0);
        let (pens, _) = gdi_pen_positions(10, "Yavin").unwrap();
        assert_eq!(pens[1], 7.0);
        // Outside cp1252 there is no table.
        assert_eq!(gdi_pen_positions(10, "Ω"), None);
        assert_eq!(cp1252('é'), Some(0xe9));
        assert_eq!(cp1252('’'), Some(0x92));
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
