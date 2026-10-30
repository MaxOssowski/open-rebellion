//! The original Game Menu Window: the pop-up the speed control and the
//! object right-click menus open (`FUN_00442860`).
//!
//! `FUN_00442a80` lays out the rows, `FUN_004aab50` paints them, and
//! `FUN_004aba60` draws each item in one of three colors that
//! `FUN_004abe10` stores: `+0x30` for a normal item, `+0x2c` for the
//! highlighted one, and `+0x34` when the item is disabled. Every builder
//! passes the faction color as the highlight, white as the normal color, and
//! gray `0x2808080` as the disabled one (`FUN_0042d190`, `FUN_004ac5c0`).
//! Recovery notes: `ghidra/notes/object-popup-menu.md` and
//! `docs/qa/2026-09-10-interface-parity-audit/evidence/2026-09-24-game-speed-recovery.md`.

use egui_macroquad::egui;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{
    CockpitFaction, CockpitLayout, CockpitViewport, STRATEGIC_LOGICAL_HEIGHT,
    STRATEGIC_LOGICAL_WIDTH, paint_gid_frame_border,
};

/// Menu text height. The Game Menu Window uses the application default font
/// (`FUN_00603850`), whose table entry is not yet recovered.
const MENU_FONT_HEIGHT: f32 = 14.0;

/// Icon column a submenu parent without an icon reserves (`FUN_004abf60`).
const SUBMENU_ICON_WIDTH: f32 = 20.0;

/// Normal item color `0x2ffffff`.
const NORMAL_COLOR: egui::Color32 = egui::Color32::WHITE;

/// Disabled item color `0x2808080`.
const DISABLED_COLOR: egui::Color32 = egui::Color32::from_rgb(0x80, 0x80, 0x80);

/// The window's fill: what lies behind it, shaded (`Class_0065bef0_vt38`
/// builds `FUN_005fddd0`'s table from `LAB_004ac550`, the sector window's
/// transform, and `FUN_004aab50`'s paint copies the screen under the menu
/// through it). Each channel `c` becomes `c / 6 + 50`
/// ([`crate::bmp_cache::sector_shade`]); blending premultiplied 50 at
/// alpha 213 leaves `50 + c x 42 / 255` of it. port: the original then takes
/// the nearest palette entry, which the blend does not.
pub(crate) const MENU_SHADE: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(50, 50, 50, 213);

/// Faction text color: Alliance `0x20000ff` (red), Empire `0x200ff00` (green).
/// The menus highlight an item in it, and the day readout is drawn in it.
#[must_use]
pub const fn faction_text_color(faction: CockpitFaction) -> egui::Color32 {
    match faction {
        CockpitFaction::Alliance => egui::Color32::from_rgb(255, 0, 0),
        CockpitFaction::Empire => egui::Color32::from_rgb(0, 255, 0),
    }
}

/// One row of a Game Menu Window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameMenuEntry<'a> {
    pub label: &'a str,
    /// STRATEGY icon named by the record's words 7 and 8.
    pub icon: Option<u32>,
    /// Cleared when the record's flags carry bit 0 (`FUN_00442590`).
    pub enabled: bool,
    /// A submenu parent (`FUN_00442590` flags 8), which widens the icon column.
    pub submenu: bool,
}

/// The color `FUN_004aba60` draws an entry's text in.
#[must_use]
pub fn entry_text_color(
    entry: &GameMenuEntry<'_>,
    highlighted: bool,
    faction: CockpitFaction,
) -> egui::Color32 {
    if !entry.enabled {
        DISABLED_COLOR
    } else if highlighted {
        faction_text_color(faction)
    } else {
        NORMAL_COLOR
    }
}

/// The width `FUN_004abf60` measures for an entry's icon column.
#[must_use]
pub fn entry_icon_width(entry: &GameMenuEntry<'_>, icon_width: Option<f32>) -> f32 {
    match icon_width {
        Some(width) => width,
        None if entry.submenu => SUBMENU_ICON_WIDTH,
        None => 0.0,
    }
}

/// Row geometry of a Game Menu Window, in logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct GameMenuGeometry {
    /// Widest item icon (`FUN_004abf60`), shared by every row.
    pub icon_column: f32,
    pub width: f32,
    pub height: f32,
    /// Top edge and height of each row.
    pub rows: Vec<(f32, f32)>,
}

/// Lay out rows as `FUN_00442a80` and `FUN_004abb80` / `FUN_004abbf0` /
/// `FUN_004abc70` do.
///
/// Each row is `icon_column + 12 + text` wide and `max(icon, text) + 4` tall.
/// Rows start two pixels down, and the window adds two pixels below the sum.
#[must_use]
pub fn game_menu_geometry(
    icon_sizes: &[(f32, f32)],
    text_sizes: &[(f32, f32)],
) -> GameMenuGeometry {
    let icon_column = icon_sizes.iter().map(|size| size.0).fold(0.0, f32::max);
    let mut width = 0.0_f32;
    let mut rows = Vec::with_capacity(text_sizes.len());
    let mut top = 2.0;
    for (index, text) in text_sizes.iter().enumerate() {
        let icon_height = icon_sizes.get(index).map_or(0.0, |size| size.1);
        width = width.max(icon_column + 12.0 + text.0);
        let height = (icon_height + 4.0).max(text.1 + 4.0);
        rows.push((top, height));
        top += height;
    }
    let height = rows.iter().map(|row| row.1).sum::<f32>() + 2.0;
    GameMenuGeometry {
        icon_column,
        width,
        height,
        rows,
    }
}

/// Place a menu at the cursor, flipping as `FUN_00442860` does when it would
/// cross the owner window's right or bottom edge.
#[must_use]
pub fn game_menu_origin(cursor: (f32, f32), size: (f32, f32), owner: (f32, f32)) -> (f32, f32) {
    let (mut x, mut y) = cursor;
    if y + size.1 > owner.1 {
        y -= size.1;
    }
    if x < 0.0 {
        x += size.0;
    } else if x + size.0 > owner.0 {
        x -= size.0;
    }
    (x, y)
}

/// Where a menu opens: the cursor point and the window that owns the menu,
/// both in 640 by 480 canvas coordinates. `FUN_00442860` flips the menu at
/// the owner's right and bottom edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameMenuPlacement {
    pub owner: CockpitViewport,
    pub point: (f32, f32),
}

impl GameMenuPlacement {
    /// A menu owned by the main frame, as the speed menu is (`FUN_0042d190`
    /// passes the command center window).
    #[must_use]
    pub const fn in_frame(point: (f32, f32)) -> Self {
        Self {
            owner: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: STRATEGIC_LOGICAL_WIDTH,
                height: STRATEGIC_LOGICAL_HEIGHT,
            },
            point,
        }
    }

    /// The menu's top-left corner for a menu of `size`.
    #[must_use]
    pub fn origin(self, size: (f32, f32)) -> (f32, f32) {
        let (x, y) = game_menu_origin(
            (self.point.0 - self.owner.x, self.point.1 - self.owner.y),
            size,
            (self.owner.width, self.owner.height),
        );
        (self.owner.x + x, self.owner.y + y)
    }
}

/// What the player did with an open menu this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMenuResponse {
    Open,
    /// The index of the enabled entry chosen; the menu closes.
    Chosen(usize),
    /// The index of the enabled submenu entry chosen; the menu closes.
    ChosenChild(usize),
    /// A press outside the menu or Escape closed it.
    Dismissed,
}

/// The submenu of one parent row: its own Game Menu Window
/// (`FUN_00442860` builds one for each parent, `FUN_004ab3a0` ties it to
/// the parent's item).
#[derive(Debug, Clone, Copy)]
pub struct GameSubmenu<'a> {
    /// The parent's row.
    pub parent: usize,
    pub entries: &'a [GameMenuEntry<'a>],
}

/// The window's timer `100` (500 ms, set when it is shown) and its
/// highlighted item `+0x118` (`FUN_004aab50`).
const SUBMENU_TIMER: f64 = 0.5;

/// What an open menu with a submenu keeps between frames.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GameMenuState {
    /// When the menu was shown, which starts its 500 ms timer.
    shown_at: Option<f64>,
    /// The highlighted row (`+0x118`) and when it became so.
    highlighted: Option<(usize, f64)>,
    /// The highlighted parent's submenu is shown.
    submenu_open: bool,
}

impl GameMenuState {
    /// Whether the highlighted parent's submenu is shown.
    #[must_use]
    pub fn submenu_shown(&self) -> bool {
        self.submenu_open
    }
}

/// The submenu arrows (`FUN_004ab3a0`): STRATEGY 10117 on the highlighted
/// parent, 10118 on the others, for side 1; 10128 and 10129 otherwise
/// (`FUN_00401060`).
#[must_use]
pub const fn submenu_arrow(faction: CockpitFaction, highlighted: bool) -> u32 {
    let base = match faction {
        CockpitFaction::Alliance => 10117,
        CockpitFaction::Empire => 10128,
    };
    if highlighted {
        base
    } else {
        base + 1
    }
}

/// Where `FUN_004ab3a0` places a submenu of `size` beside its parent window
/// `parent` (x, y, width, height) for the parent row spanning `row_top` to
/// `row_bottom` within it: on the parent's left, overlapping it by three
/// pixels, or on its right when that would cross the frame's left edge;
/// three pixels above the row when it fits above the parent's bottom, else
/// ending three pixels below the row.
#[must_use]
pub fn submenu_origin(
    parent: (f32, f32, f32, f32),
    row_top: f32,
    row_bottom: f32,
    size: (f32, f32),
) -> (f32, f32) {
    let (px, py, pw, ph) = parent;
    let mut x = px - size.0 + 3.0;
    if x < 0.0 {
        x = px + pw - 3.0;
    }
    let y = if row_top + size.1 + py < py + ph {
        row_top - 3.0 + py
    } else {
        py + row_bottom - size.1 + 3.0
    };
    (x, y)
}

/// Whether the 500 ms timer has fired since `since` (`WM_TIMER`, which
/// shows the highlighted parent's submenu).
fn timer_fired(shown_at: f64, since: f64, now: f64) -> bool {
    let ticks = ((since - shown_at) / SUBMENU_TIMER).floor() + 1.0;
    now >= shown_at + ticks * SUBMENU_TIMER
}

/// Draw a Game Menu Window at `placement`.
///
/// Rows highlight under the pointer, a left click chooses an enabled row
/// (`FUN_004424c0`), and a press outside or Escape closes the menu.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep the menu's owner, faction, placement and entries explicit at this drawing boundary."
)]
pub fn draw_game_menu(
    ctx: &egui::Context,
    id: egui::Id,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    placement: GameMenuPlacement,
    entries: &[GameMenuEntry<'_>],
    input_enabled: bool,
) -> GameMenuResponse {
    draw_game_menu_with_submenu(
        ctx,
        id,
        cache,
        layout,
        faction,
        placement,
        entries,
        None,
        &mut GameMenuState::default(),
        input_enabled,
    )
}

/// Measure a window's rows (`FUN_00442a80`), in logical pixels.
fn measure(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    entries: &[GameMenuEntry<'_>],
    scale: f32,
    font: &egui::FontId,
) -> (GameMenuGeometry, Vec<(f32, f32)>) {
    let text_sizes: Vec<(f32, f32)> = entries
        .iter()
        .map(|entry| {
            let size = ctx
                .fonts(|fonts| {
                    fonts.layout_no_wrap(entry.label.to_owned(), font.clone(), NORMAL_COLOR)
                })
                .size();
            (size.x / scale, size.y / scale)
        })
        .collect();
    let icon_sizes: Vec<(f32, f32)> = entries
        .iter()
        .map(|entry| {
            let icon = entry.icon.and_then(|id| {
                cache
                    .get(ctx, DllSource::Strategy, id)
                    .map(|texture| texture.size_vec2())
            });
            (
                entry_icon_width(entry, icon.map(|size| size.x)),
                icon.map_or(0.0, |size| size.y),
            )
        })
        .collect();
    (game_menu_geometry(&icon_sizes, &text_sizes), text_sizes)
}

/// One window's frame: its screen rectangle, the enabled row under the
/// pointer, and the enabled row clicked.
struct WindowFrame {
    rect: egui::Rect,
    hovered: Option<usize>,
    clicked: Option<usize>,
}

/// Paint one Game Menu Window at the canvas point `at` (`FUN_004aab50`).
#[expect(
    clippy::too_many_arguments,
    reason = "One window's owner, faction, rows, highlight and input at this drawing boundary."
)]
fn menu_window(
    ctx: &egui::Context,
    id: egui::Id,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    at: (f32, f32),
    entries: &[GameMenuEntry<'_>],
    measured: &(GameMenuGeometry, Vec<(f32, f32)>),
    highlighted: impl Fn(usize, bool) -> bool,
    input_enabled: bool,
) -> WindowFrame {
    let (geometry, text_sizes) = measured;
    let scale = layout.scale.max(0.5);
    let font = egui::FontId::proportional(MENU_FONT_HEIGHT * scale);
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let origin = egui::pos2(
        layout.canvas.x + at.0 * layout.scale,
        layout.canvas.y + at.1 * layout.scale,
    );
    let menu_size = egui::vec2(geometry.width * scale, geometry.height * scale);

    // FUN_00442860 opens a pop-up over every child window. The modeless
    // windows share Foreground and a focused one raises itself each frame
    // (`system_window.rs`); the Message Index, the Finders and the mission
    // dialog take Tooltip and raise themselves there. So the menu takes
    // the order above them all.
    let shown = egui::Area::new(id)
        .order(egui::Order::Debug)
        .fade_in(false)
        .fixed_pos(origin)
        .show(ctx, |ui| {
            if !input_enabled {
                ui.disable();
            }
            let (rect, _) = ui.allocate_exact_size(menu_size, egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, MENU_SHADE);
            let mut hovered = None;
            let mut clicked = None;
            for (index, ((entry, (top, height)), text)) in entries
                .iter()
                .zip(geometry.rows.iter().copied())
                .zip(text_sizes.iter().copied())
                .enumerate()
            {
                let row = egui::Rect::from_min_size(
                    rect.min + egui::vec2(0.0, top * scale),
                    egui::vec2(rect.width(), height * scale),
                );
                let sense = if entry.enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                };
                let response = ui.interact(row, id.with(("entry", index)), sense);
                // WM_MOUSEMOVE: PtInRect on each enabled item (+0x58).
                let under = entry.enabled && pointer.is_some_and(|p| row.contains(p));
                if let Some(texture) = entry
                    .icon
                    .and_then(|icon| cache.get(ctx, DllSource::Strategy, icon))
                {
                    let size = texture.size_vec2() * scale;
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_min_size(row.min + egui::vec2(6.0 * scale, 0.0), size),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                let lit = entry.enabled && highlighted(index, under);
                if entry.submenu && entry.icon.is_none() {
                    // FUN_004aab50 draws the arrow at the item's left + 6.
                    if let Some(texture) =
                        cache.get(ctx, DllSource::Strategy, submenu_arrow(faction, lit))
                    {
                        let size = texture.size_vec2() * scale;
                        ui.painter().image(
                            texture.id(),
                            egui::Rect::from_min_size(row.min + egui::vec2(6.0 * scale, 0.0), size),
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                ui.painter().text(
                    egui::pos2(
                        row.min.x + (geometry.icon_column + 6.0) * scale,
                        row.min.y + ((height - text.1) / 2.0) * scale,
                    ),
                    egui::Align2::LEFT_TOP,
                    entry.label,
                    font.clone(),
                    entry_text_color(entry, lit, faction),
                );
                if under {
                    hovered = Some(index);
                }
                if entry.enabled && response.clicked() {
                    clicked = Some(index);
                }
            }
            paint_gid_frame_border(ui, cache, rect, scale);
            (hovered, clicked)
        });
    WindowFrame {
        rect: shown.response.rect,
        hovered: shown.inner.0,
        clicked: shown.inner.1,
    }
}

/// Draw a Game Menu Window at `placement` with an optional submenu
/// (`FUN_004aab50`): the pointer highlights an enabled row; each tick of
/// the window's 500 ms timer shows the highlighted parent's submenu, and
/// another highlight hides it; a click on a parent does nothing, on an
/// enabled row of either window chooses it; Escape leaves an open submenu
/// first, then closes the menu, as a press outside both does.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep the menu's owner, faction, placement, entries and state explicit at this drawing boundary."
)]
pub fn draw_game_menu_with_submenu(
    ctx: &egui::Context,
    id: egui::Id,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    placement: GameMenuPlacement,
    entries: &[GameMenuEntry<'_>],
    submenu: Option<GameSubmenu<'_>>,
    state: &mut GameMenuState,
    input_enabled: bool,
) -> GameMenuResponse {
    let scale = layout.scale.max(0.5);
    let font = egui::FontId::proportional(MENU_FONT_HEIGHT * scale);
    let now = ctx.input(|input| input.time);
    let shown_at = *state.shown_at.get_or_insert(now);
    let measured = measure(ctx, cache, entries, scale, &font);
    let (x, y) = placement.origin((measured.0.width, measured.0.height));
    let highlighted = state.highlighted.map(|(row, _)| row);
    let frame = menu_window(
        ctx,
        id,
        cache,
        layout,
        faction,
        (x, y),
        entries,
        &measured,
        |row, hovered| hovered || (submenu.is_some() && highlighted == Some(row)),
        input_enabled,
    );
    // WM_MOUSEMOVE: inside the window the enabled row under the pointer
    // becomes the highlight, or none; outside it the highlight stays.
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    if pointer.is_some_and(|p| frame.rect.contains(p)) {
        let row = frame.hovered;
        if row != highlighted {
            state.highlighted = row.map(|r| (r, now));
            state.submenu_open = false;
        }
    }
    let parent_lit = submenu.filter(|sub| state.highlighted.map(|(r, _)| r) == Some(sub.parent));
    if let (Some(_), Some((_, since))) = (parent_lit, state.highlighted) {
        if !state.submenu_open && timer_fired(shown_at, since, now) {
            state.submenu_open = true;
        }
    }
    if parent_lit.is_some() && state.submenu_open {
        ctx.request_repaint();
    } else if parent_lit.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    let mut child = None;
    if let (Some(sub), true) = (parent_lit, state.submenu_open) {
        let child_measured = measure(ctx, cache, sub.entries, scale, &font);
        let (top, height) = measured.0.rows[sub.parent];
        let at = submenu_origin(
            (x, y, measured.0.width, measured.0.height),
            top,
            top + height,
            (child_measured.0.width, child_measured.0.height),
        );
        child = Some(menu_window(
            ctx,
            id.with("submenu"),
            cache,
            layout,
            faction,
            at,
            sub.entries,
            &child_measured,
            |_, hovered| hovered,
            input_enabled,
        ));
    }
    if let Some(index) = child.as_ref().and_then(|c| c.clicked) {
        return GameMenuResponse::ChosenChild(index);
    }
    if let Some(index) = frame.clicked {
        // A parent with its submenu (item +0x3c) issues nothing.
        if submenu.is_none_or(|sub| sub.parent != index) {
            return GameMenuResponse::Chosen(index);
        }
    }
    if !input_enabled {
        return GameMenuResponse::Open;
    }
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        if child.is_some() {
            // The parent window takes the focus back; its timer shows the
            // submenu again on a later tick.
            state.submenu_open = false;
            state.highlighted = state.highlighted.map(|(row, _)| (row, now));
            return GameMenuResponse::Open;
        }
        return GameMenuResponse::Dismissed;
    }
    let outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input.pointer.interact_pos().is_some_and(|p| {
                !frame.rect.contains(p) && child.as_ref().is_none_or(|c| !c.rect.contains(p))
            })
    });
    if outside {
        GameMenuResponse::Dismissed
    } else {
        GameMenuResponse::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(enabled: bool, submenu: bool) -> GameMenuEntry<'static> {
        GameMenuEntry {
            label: "Mission",
            icon: None,
            enabled,
            submenu,
        }
    }

    const MENU_ID: &str = "game_menu_test";

    #[test]
    fn the_menu_opens_above_a_dialog_shown_after_it() {
        // FUN_00442860's pop-up is the topmost window; the mission dialog
        // shares the order above the modeless windows (mission_dialog.rs).
        let layout = layout(1.0);
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let dialog = egui::Id::new("dialog_above_windows");
        let entries = [labelled("Mission", true)];
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let _ = draw_game_menu(
                    ctx,
                    egui::Id::new(MENU_ID),
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    GameMenuPlacement::in_frame((100.0, 100.0)),
                    &entries,
                    true,
                );
                egui::Area::new(dialog)
                    .order(egui::Order::Tooltip)
                    .fixed_pos(egui::Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.allocate_response(egui::vec2(400.0, 400.0), egui::Sense::click());
                    });
            });
        }
        let menu = ctx
            .memory(|memory| memory.area_rect(egui::Id::new(MENU_ID)))
            .expect("the menu is laid out");

        assert_eq!(
            ctx.layer_id_at(menu.center()).map(|layer| layer.id),
            Some(egui::Id::new(MENU_ID))
        );
    }

    #[test]
    fn the_menu_stays_above_a_window_that_raises_itself_every_frame() {
        // FUN_00442860 makes the Game Menu Window a pop-up over every child
        // window; a focused system window raises itself each frame.
        let layout = layout(1.0);
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let window = egui::Id::new("raising_window");
        let entries = [labelled("Mission", true)];
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, window));
                egui::Area::new(window)
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.allocate_response(egui::vec2(400.0, 400.0), egui::Sense::click());
                    });
                let _ = draw_game_menu(
                    ctx,
                    egui::Id::new(MENU_ID),
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    GameMenuPlacement::in_frame((100.0, 100.0)),
                    &entries,
                    true,
                );
            });
        }
        let menu = ctx
            .memory(|memory| memory.area_rect(egui::Id::new(MENU_ID)))
            .expect("the menu is laid out");

        assert_eq!(
            ctx.layer_id_at(menu.center()).map(|layer| layer.id),
            Some(egui::Id::new(MENU_ID))
        );
    }

    fn labelled(label: &'static str, enabled: bool) -> GameMenuEntry<'static> {
        GameMenuEntry {
            label,
            icon: None,
            enabled,
            submenu: false,
        }
    }

    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// What one test run saw: the last frame's response, the menu rectangle,
    /// and the text it painted, as (label, top-left corner, size, color).
    struct Run {
        response: GameMenuResponse,
        rect: egui::Rect,
        texts: Vec<(String, egui::Pos2, egui::Vec2, egui::Color32)>,
    }

    fn layout(scale: f32) -> CockpitLayout {
        crate::cockpit::CockpitState::new(CockpitFaction::Alliance)
            .layout_for(640.0 * scale, 480.0 * scale)
    }

    /// Open a menu of `entries` at (100, 100) on a 640 by 480 owner drawn at
    /// `scale`, run two idle frames so the area lays out, then one frame per
    /// entry of `frames` with the pointer at the point `at` returns for the
    /// laid-out menu rectangle.
    fn run(
        scale: f32,
        entries: &[GameMenuEntry<'_>],
        at: impl Fn(egui::Rect) -> egui::Pos2,
        frames: Vec<Vec<egui::Event>>,
    ) -> Run {
        let layout = layout(scale);
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let id = egui::Id::new(MENU_ID);
        let mut response = GameMenuResponse::Open;
        let mut rect = egui::Rect::NOTHING;
        let mut texts = Vec::new();
        for (frame, extra) in [vec![], vec![]].into_iter().chain(frames).enumerate() {
            let pointer = if frame >= 2 {
                rect = ctx
                    .memory(|memory| memory.area_rect(id))
                    .expect("the menu area is laid out");
                at(rect)
            } else {
                egui::Pos2::ZERO
            };
            let mut events = vec![egui::Event::PointerMoved(pointer)];
            events.extend(extra.into_iter().map(|event| match event {
                egui::Event::PointerButton { pressed, .. } => pointer_button(pointer, pressed),
                other => other,
            }));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0 * scale, 520.0 * scale),
                )),
                events,
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                response = draw_game_menu(
                    ctx,
                    id,
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    GameMenuPlacement::in_frame((100.0, 100.0)),
                    entries,
                    true,
                );
            });
            texts = output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Text(text) => Some((
                        text.galley.text().to_owned(),
                        text.pos,
                        text.galley.size(),
                        text.fallback_color,
                    )),
                    _ => None,
                })
                .collect();
        }
        Run {
            response,
            rect,
            texts,
        }
    }

    fn drive(
        entries: &[GameMenuEntry<'_>],
        at: impl Fn(egui::Rect) -> egui::Pos2,
        frames: Vec<Vec<egui::Event>>,
    ) -> GameMenuResponse {
        run(1.0, entries, at, frames).response
    }

    /// The top and height of row `index` of `count` equal rows drawn at
    /// `scale`: two pixels of top padding, then the rows, as
    /// `game_menu_geometry` lays them out.
    fn row_span(rect: egui::Rect, scale: f32, index: usize, count: usize) -> (f32, f32) {
        let row = (rect.height() - 2.0 * scale) / count as f32;
        (rect.min.y + 2.0 * scale + row * index as f32, row)
    }

    fn row_center(rect: egui::Rect, scale: f32, index: usize, count: usize) -> egui::Pos2 {
        let (top, row) = row_span(rect, scale, index, count);
        egui::pos2(rect.center().x, top + row / 2.0)
    }

    fn click() -> Vec<Vec<egui::Event>> {
        let at = egui::Pos2::ZERO;
        vec![
            vec![pointer_button(at, true)],
            vec![pointer_button(at, false)],
        ]
    }

    #[test]
    fn a_click_on_an_enabled_row_chooses_that_row() {
        // FUN_004424c0: a left release on an item reports its kind to the owner.
        let entries = [
            labelled("Move", true),
            labelled("Mission", true),
            labelled("Encyclopedia", true),
        ];
        for index in 0..entries.len() {
            let response = drive(&entries, |rect| row_center(rect, 1.0, index, 3), click());
            assert_eq!(response, GameMenuResponse::Chosen(index));
        }
    }

    #[test]
    fn a_click_on_a_disabled_row_chooses_nothing_and_keeps_the_menu_open() {
        // FUN_004aba60: a disabled item (flags bit 0) neither highlights nor reports.
        let entries = [labelled("Move", false), labelled("Mission", true)];
        let response = drive(&entries, |rect| row_center(rect, 1.0, 0, 2), click());
        assert_eq!(response, GameMenuResponse::Open);
    }

    #[test]
    fn a_press_outside_the_menu_dismisses_it() {
        // FUN_00442860's window closes on a press outside it.
        let entries = [labelled("Mission", true)];
        let response = drive(
            &entries,
            |rect| rect.max + egui::vec2(20.0, 20.0),
            vec![vec![pointer_button(egui::Pos2::ZERO, true)]],
        );
        assert_eq!(response, GameMenuResponse::Dismissed);
    }

    #[test]
    fn hovering_the_menu_without_a_press_keeps_it_open() {
        let entries = [labelled("Mission", true)];
        let response = drive(&entries, |rect| row_center(rect, 1.0, 0, 1), vec![vec![]]);
        assert_eq!(response, GameMenuResponse::Open);
    }

    #[test]
    fn escape_dismisses_the_menu() {
        // port: Escape closes the menu, as the speed menu's port already did.
        let entries = [labelled("Mission", true)];
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        };
        let response = drive(
            &entries,
            |rect| row_center(rect, 1.0, 0, 1),
            vec![vec![escape]],
        );
        assert_eq!(response, GameMenuResponse::Dismissed);
    }

    #[test]
    fn the_menu_opens_at_the_anchor_and_scales_with_the_owner() {
        // FUN_00442860 places the menu's top-left corner at the click point.
        let entries = [labelled("Move", true), labelled("Mission", true)];
        let small = run(1.0, &entries, |rect| rect.center(), vec![vec![]]);
        let large = run(2.0, &entries, |rect| rect.center(), vec![vec![]]);
        for (scale, seen) in [(1.0, &small), (2.0, &large)] {
            let layout = layout(scale);
            assert_eq!(
                seen.rect.min,
                egui::pos2(
                    layout.canvas.x + 100.0 * layout.scale,
                    layout.canvas.y + 100.0 * layout.scale,
                ),
                "scale {scale}"
            );
        }
        // Heights scale exactly; egui measures text widths at the scaled font
        // size, which rounds differently from twice the 14-pixel width.
        assert_eq!(large.rect.height(), small.rect.height() * 2.0);
        assert!(
            (large.rect.width() - small.rect.width() * 2.0).abs() <= 2.0,
            "{} vs {}",
            large.rect.width(),
            small.rect.width()
        );
    }

    #[test]
    fn rows_are_chosen_by_their_place_at_any_scale() {
        let entries = [
            labelled("Move", true),
            labelled("Mission", true),
            labelled("Encyclopedia", true),
        ];
        // Near each row's top, middle and bottom.
        for index in 0..entries.len() {
            for depth in [0.1, 0.5, 0.9] {
                let seen = run(
                    2.0,
                    &entries,
                    |rect| {
                        let (top, row) = row_span(rect, 2.0, index, 3);
                        egui::pos2(rect.center().x, top + row * depth)
                    },
                    click(),
                );
                assert_eq!(
                    seen.response,
                    GameMenuResponse::Chosen(index),
                    "row {index} at depth {depth}"
                );
            }
        }
    }

    #[test]
    fn text_sits_six_pixels_in_and_centred_in_its_row() {
        // FUN_004aba60: text starts after the icon column plus 6 and is
        // vertically centred in the row.
        for scale in [1.0, 2.0] {
            let entries = [labelled("Move", true), labelled("Mission", true)];
            let seen = run(
                scale,
                &entries,
                |rect| rect.max + egui::vec2(9.0, 9.0),
                vec![vec![]],
            );
            for (index, label) in ["Move", "Mission"].into_iter().enumerate() {
                let (_, pos, size, _) = seen
                    .texts
                    .iter()
                    .find(|(text, ..)| text == label)
                    .unwrap_or_else(|| panic!("{label} is painted"));
                let (top, row) = row_span(seen.rect, scale, index, 2);
                assert_eq!(pos.x, seen.rect.min.x + 6.0 * scale, "{label} at {scale}");
                assert!(
                    (pos.y - (top + (row - size.y) / 2.0)).abs() < 0.01,
                    "{label} at {scale}: top {} in row {top}+{row}, height {}",
                    pos.y,
                    size.y
                );
            }
            // The widest row sets the width: icon column, 12 pixels, text.
            let widest = seen
                .texts
                .iter()
                .map(|(.., size, _)| size.x)
                .fold(0.0, f32::max);
            assert!(
                (seen.rect.width() - (widest + 12.0 * scale)).abs() < 0.01,
                "width {} for text {widest} at {scale}",
                seen.rect.width()
            );
        }
    }

    #[test]
    fn the_hovered_enabled_row_is_painted_in_the_faction_color() {
        // FUN_004aba60: the highlighted item uses +0x2c, the faction color.
        let entries = [
            labelled("Move", false),
            labelled("Mission", true),
            labelled("Status", true),
        ];
        let color = |seen: &Run, label: &str| {
            seen.texts
                .iter()
                .find(|(text, ..)| text == label)
                .map(|(.., color)| *color)
        };
        let hover_mission = run(
            1.0,
            &entries,
            |rect| row_center(rect, 1.0, 1, 3),
            vec![vec![]],
        );
        assert_eq!(
            color(&hover_mission, "Mission"),
            Some(egui::Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(color(&hover_mission, "Status"), Some(egui::Color32::WHITE));
        let hover_move = run(
            1.0,
            &entries,
            |rect| row_center(rect, 1.0, 0, 3),
            vec![vec![]],
        );
        assert_eq!(color(&hover_move, "Move"), Some(DISABLED_COLOR));
        assert_eq!(color(&hover_move, "Mission"), Some(egui::Color32::WHITE));
    }

    #[test]
    fn hovering_outside_without_a_press_keeps_the_menu_open() {
        let entries = [labelled("Mission", true)];
        let response = drive(
            &entries,
            |rect| rect.max + egui::vec2(20.0, 20.0),
            vec![vec![]],
        );
        assert_eq!(response, GameMenuResponse::Open);
    }

    #[test]
    fn items_are_white_highlighted_in_the_faction_color_and_gray_when_disabled() {
        // Source: FUN_004aba60 draws +0x30 normally, +0x2c highlighted and +0x34 disabled;
        // FUN_004abe10 stores (faction, 0x2ffffff, 0x2808080) from FUN_00442590.
        let enabled = entry(true, false);
        assert_eq!(
            entry_text_color(&enabled, false, CockpitFaction::Alliance),
            egui::Color32::WHITE
        );
        assert_eq!(
            entry_text_color(&enabled, true, CockpitFaction::Alliance),
            egui::Color32::from_rgb(255, 0, 0)
        );
        assert_eq!(
            entry_text_color(&enabled, true, CockpitFaction::Empire),
            egui::Color32::from_rgb(0, 255, 0)
        );
        let disabled = entry(false, false);
        for highlighted in [false, true] {
            assert_eq!(
                entry_text_color(&disabled, highlighted, CockpitFaction::Empire),
                egui::Color32::from_rgb(0x80, 0x80, 0x80)
            );
        }
    }

    #[test]
    fn a_submenu_parent_without_an_icon_reserves_twenty_pixels() {
        // Source: FUN_004abf60 returns 0x14 for an item whose flags carry 8.
        assert_eq!(entry_icon_width(&entry(true, true), None), 20.0);
        assert_eq!(entry_icon_width(&entry(true, false), None), 0.0);
        assert_eq!(entry_icon_width(&entry(true, true), Some(16.0)), 16.0);
    }

    #[test]
    fn menu_rows_follow_native_width_and_height_rules() {
        let icons = [(16.0, 10.0); 5];
        let texts = [
            (30.0, 16.0),
            (55.0, 16.0),
            (24.0, 16.0),
            (44.0, 16.0),
            (26.0, 16.0),
        ];
        let geometry = game_menu_geometry(&icons, &texts);
        assert_eq!(geometry.icon_column, 16.0);
        // Widest row: 16 + 12 + 55.
        assert_eq!(geometry.width, 83.0);
        assert_eq!(
            geometry.rows,
            vec![
                (2.0, 20.0),
                (22.0, 20.0),
                (42.0, 20.0),
                (62.0, 20.0),
                (82.0, 20.0)
            ]
        );
        assert_eq!(geometry.height, 102.0);
    }

    #[test]
    fn short_text_rows_keep_icon_height_plus_padding() {
        let geometry = game_menu_geometry(&[(16.0, 10.0)], &[(10.0, 8.0)]);
        assert_eq!(geometry.rows, vec![(2.0, 14.0)]);
    }

    #[test]
    fn menu_touching_an_owner_edge_does_not_flip() {
        // FUN_00442860 flips only when the menu strictly crosses the edge.
        let owner = (640.0, 480.0);
        assert_eq!(
            game_menu_origin((557.0, 378.0), (83.0, 102.0), owner),
            (557.0, 378.0)
        );
        assert_eq!(
            game_menu_origin((558.0, 379.0), (83.0, 102.0), owner),
            (475.0, 277.0)
        );
    }

    #[test]
    fn menu_left_of_the_owner_moves_right_by_its_width() {
        // FUN_00442860: a negative x gains the menu width; zero stays put.
        let owner = (640.0, 480.0);
        assert_eq!(
            game_menu_origin((-10.0, 26.0), (83.0, 102.0), owner),
            (73.0, 26.0)
        );
        assert_eq!(
            game_menu_origin((0.0, 26.0), (83.0, 102.0), owner),
            (0.0, 26.0)
        );
    }

    #[test]
    fn a_menu_flips_at_its_owner_window_not_the_canvas() {
        // FUN_004ac5c0 maps the point into the galaxy view, which owns the
        // menu (FUN_00442380); FUN_00442860 flips at the owner's edges.
        let galaxy = GameMenuPlacement {
            owner: CockpitViewport {
                x: 55.0,
                y: 40.0,
                width: 485.0,
                height: 350.0,
            },
            point: (500.0, 360.0),
        };
        assert_eq!(galaxy.origin((83.0, 102.0)), (417.0, 258.0));
        let inside = GameMenuPlacement {
            point: (120.0, 60.0),
            ..galaxy
        };
        assert_eq!(inside.origin((83.0, 102.0)), (120.0, 60.0));
        // The same point in the frame does not cross its edges.
        assert_eq!(
            GameMenuPlacement::in_frame((500.0, 360.0)).origin((83.0, 102.0)),
            (500.0, 360.0)
        );
    }

    #[test]
    fn menu_flips_away_from_right_and_bottom_owner_edges() {
        let owner = (640.0, 480.0);
        assert_eq!(
            game_menu_origin((120.0, 26.0), (83.0, 102.0), owner),
            (120.0, 26.0)
        );
        assert_eq!(
            game_menu_origin((600.0, 26.0), (83.0, 102.0), owner),
            (517.0, 26.0)
        );
        assert_eq!(
            game_menu_origin((120.0, 420.0), (83.0, 102.0), owner),
            (120.0, 318.0)
        );
    }

    #[test]
    fn a_submenu_opens_left_of_its_parent_beside_the_row_or_flips() {
        // FUN_004ab3a0: x = parent.x - width + 3, or parent.x + parent.width
        // - 3 when that is negative; y = parent.y + row.top - 3 when the
        // submenu fits above parent.y + parent.height, else parent.y +
        // row.bottom - height + 3.
        assert_eq!(
            submenu_origin((200.0, 100.0, 120.0, 150.0), 40.0, 58.0, (100.0, 60.0)),
            (103.0, 137.0)
        );
        assert_eq!(
            submenu_origin((50.0, 100.0, 120.0, 150.0), 40.0, 58.0, (100.0, 60.0)),
            (167.0, 137.0)
        );
        // 120 + 40 = 160 is not below 150: the submenu ends at the row.
        assert_eq!(
            submenu_origin((200.0, 100.0, 120.0, 150.0), 120.0, 138.0, (100.0, 40.0)),
            (103.0, 201.0)
        );
        // Both tests are strict (`X < 0`, `... < iVar1`): a submenu that
        // reaches the frame's left edge exactly stays on the left, and one
        // that ends exactly at the parent's bottom moves up.
        assert_eq!(
            submenu_origin((97.0, 100.0, 120.0, 150.0), 40.0, 58.0, (100.0, 60.0)),
            (0.0, 137.0)
        );
        assert_eq!(
            submenu_origin((200.0, 100.0, 120.0, 150.0), 90.0, 108.0, (100.0, 60.0)),
            (103.0, 151.0)
        );
    }

    #[test]
    fn the_submenu_waits_for_the_window_s_next_half_second_tick() {
        // FUN_004aab50: WM_SHOWWINDOW sets timer 100 to 500 ms; WM_TIMER
        // shows the highlighted parent's submenu.
        assert!(!timer_fired(0.0, 0.1, 0.49));
        assert!(timer_fired(0.0, 0.1, 0.5));
        assert!(!timer_fired(0.0, 0.5, 0.9));
        assert!(timer_fired(0.0, 0.5, 1.0));
        assert!(timer_fired(2.0, 2.7, 3.0));
    }

    #[test]
    fn the_submenu_arrows_follow_the_side_and_the_highlight() {
        // FUN_00442860 -> FUN_004ab3a0: 0x2785/0x2786 for side 1, + 0xb else.
        assert_eq!(submenu_arrow(CockpitFaction::Alliance, true), 10117);
        assert_eq!(submenu_arrow(CockpitFaction::Alliance, false), 10118);
        assert_eq!(submenu_arrow(CockpitFaction::Empire, true), 10128);
        assert_eq!(submenu_arrow(CockpitFaction::Empire, false), 10129);
    }

    /// One frame of a submenu run: when, where the pointer is (by a
    /// function of the menu and submenu rectangles), and extra events.
    struct Step {
        time: f64,
        at: fn(egui::Rect, Option<egui::Rect>) -> egui::Pos2,
        events: Vec<egui::Event>,
    }

    /// Draw a three-row menu (Move, the parent, Status) whose middle row
    /// opens a two-row submenu (Military, a disabled Destroy), one frame
    /// per step; the responses and whether the submenu was laid out.
    fn run_submenu(steps: Vec<Step>) -> Vec<(GameMenuResponse, bool)> {
        let layout = layout(1.0);
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let id = egui::Id::new(MENU_ID);
        let entries = [
            labelled("Move", true),
            GameMenuEntry {
                label: "Planetary Bombardment",
                icon: None,
                enabled: true,
                submenu: true,
            },
            labelled("Status", true),
        ];
        let children = [labelled("Military", true), labelled("Destroy", false)];
        let mut state = GameMenuState::default();
        let mut out = Vec::new();
        let mut child_rect: Option<egui::Rect> = None;
        for (index, step) in [
            Step {
                time: 0.0,
                at: |_, _| egui::Pos2::ZERO,
                events: vec![],
            },
            Step {
                time: 0.01,
                at: |_, _| egui::Pos2::ZERO,
                events: vec![],
            },
        ]
        .into_iter()
        .chain(steps)
        .enumerate()
        {
            let menu = ctx.memory(|memory| memory.area_rect(id));
            let pointer = menu.map_or(egui::Pos2::ZERO, |menu| (step.at)(menu, child_rect));
            let mut events = vec![egui::Event::PointerMoved(pointer)];
            events.extend(step.events.into_iter().map(|event| match event {
                egui::Event::PointerButton { pressed, .. } => pointer_button(pointer, pressed),
                other => other,
            }));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                time: Some(step.time),
                events,
                ..Default::default()
            };
            let mut response = GameMenuResponse::Open;
            let _ = ctx.run(input, |ctx| {
                response = draw_game_menu_with_submenu(
                    ctx,
                    id,
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    GameMenuPlacement::in_frame((300.0, 100.0)),
                    &entries,
                    Some(GameSubmenu {
                        parent: 1,
                        entries: &children,
                    }),
                    &mut state,
                    true,
                );
            });
            let open = state.submenu_shown();
            child_rect = open
                .then(|| ctx.memory(|memory| memory.area_rect(id.with("submenu"))))
                .flatten();
            if index >= 2 {
                out.push((response, open));
            }
        }
        out
    }

    fn on_parent(menu: egui::Rect, _: Option<egui::Rect>) -> egui::Pos2 {
        row_center(menu, 1.0, 1, 3)
    }

    fn on_move(menu: egui::Rect, _: Option<egui::Rect>) -> egui::Pos2 {
        row_center(menu, 1.0, 0, 3)
    }

    fn on_child(_: egui::Rect, child: Option<egui::Rect>) -> egui::Pos2 {
        child.map_or(egui::Pos2::ZERO, |child| row_center(child, 1.0, 0, 2))
    }

    fn on_disabled_child(_: egui::Rect, child: Option<egui::Rect>) -> egui::Pos2 {
        child.map_or(egui::Pos2::ZERO, |child| row_center(child, 1.0, 1, 2))
    }

    fn step(time: f64, at: fn(egui::Rect, Option<egui::Rect>) -> egui::Pos2) -> Step {
        Step {
            time,
            at,
            events: vec![],
        }
    }

    fn press(time: f64, at: fn(egui::Rect, Option<egui::Rect>) -> egui::Pos2, down: bool) -> Step {
        Step {
            time,
            at,
            events: vec![pointer_button(egui::Pos2::ZERO, down)],
        }
    }

    #[test]
    fn hovering_a_parent_opens_its_submenu_on_the_timer_and_a_child_is_chosen() {
        // FUN_004aab50: the highlighted parent's submenu shows on the next
        // 500 ms tick; a left release on a child posts its kind.
        let out = run_submenu(vec![
            step(0.1, on_parent),
            step(0.3, on_parent),
            step(0.5, on_parent),
            step(0.6, on_child),
            press(0.7, on_child, true),
            press(0.8, on_child, false),
        ]);
        let opened: Vec<bool> = out.iter().map(|(_, open)| *open).collect();
        assert_eq!(opened[..4], [false, false, true, true]);
        assert_eq!(out[5].0, GameMenuResponse::ChosenChild(0));
    }

    #[test]
    fn a_click_on_the_parent_or_a_disabled_child_chooses_nothing() {
        // FUN_004aab50: WM_LBUTTONUP on an item with a submenu (+0x3c)
        // returns; a disabled item (+0x58 clear) never highlights.
        let out = run_submenu(vec![
            press(0.1, on_parent, true),
            press(0.2, on_parent, false),
            step(0.5, on_parent),
            step(0.55, on_parent),
            press(0.6, on_disabled_child, true),
            press(0.7, on_disabled_child, false),
        ]);
        assert!(out
            .iter()
            .all(|(response, _)| *response == GameMenuResponse::Open));
        assert!(out[5].1);
    }

    #[test]
    fn another_highlight_hides_the_submenu_and_escape_leaves_it_first() {
        // FUN_004aab50: a repaint hides the submenus of items not
        // highlighted; Escape returns focus to the parent menu, and only
        // then closes it.
        let escape = |time, at| Step {
            time,
            at,
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        };
        let out = run_submenu(vec![
            step(0.1, on_parent),
            step(0.5, on_parent),
            step(0.6, on_move),
            step(0.7, on_parent),
            step(1.0, on_parent),
            escape(1.1, on_child),
            escape(1.2, on_parent),
        ]);
        let opened: Vec<bool> = out.iter().map(|(_, open)| *open).collect();
        assert_eq!(opened, [false, true, false, false, true, false, false]);
        assert_eq!(out[5].0, GameMenuResponse::Open);
        assert_eq!(out[6].0, GameMenuResponse::Dismissed);
    }
}
