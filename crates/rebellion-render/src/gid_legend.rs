//! The galaxy view's legend window and the button that restores it
//! (`ghidra/notes/gid-markers.md`).
//!
//! Choosing a display (`FUN_00426d00`) opens the legend at its kept
//! position and shows the legend button (STRATEGY 10168, `FUN_00427270`).
//! The legend's close button hides it and leaves the button
//! (`FUN_00422ce0`, message `0x405` case 3); the button (command `0x134`)
//! shows the legend again and hides itself. Display Off closes both.
//!
//! `FUN_004522f0` paints the window from one STRATEGY.DLL layout
//! (`rebellion_core::gid_legend`), frames it (`FUN_00607740`) and adds the
//! close button; the galaxy shows through the rest (`FUN_00452240`).

use std::sync::RwLock;

use egui_macroquad::egui;
use rebellion_core::gid_legend::{GidLegend, GidLegendItem, GidLegends};

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{
    CockpitFaction, CockpitLayout, GidMode, STRATEGIC_LOGICAL_HEIGHT, STRATEGIC_LOGICAL_WIDTH,
};

static GID_LEGENDS: RwLock<Option<GidLegends>> = RwLock::new(None);

/// Install the legend layouts (`rebellion_data::gid_legends`).
pub fn set_gid_legends(legends: GidLegends) {
    if let Ok(mut slot) = GID_LEGENDS.write() {
        *slot = Some(legends);
    }
}

fn legend(resource: u16) -> Option<GidLegend> {
    GID_LEGENDS.read().ok()?.as_ref()?.get(&resource).cloned()
}

fn legend_size(resource: u16) -> Option<egui::Vec2> {
    let legends = GID_LEGENDS.read().ok()?;
    let legend = legends.as_ref()?.get(&resource)?;
    Some(egui::vec2(
        f32::from(legend.width),
        f32::from(legend.height),
    ))
}

/// The legend button (`FUN_00427270`): a 30 x 30 control at (55, 50) for the
/// Alliance and (113, 50) for the Empire, STRATEGY 10168 (`0x27b8`).
pub const LEGEND_BUTTON: u32 = crate::bmp_cache::resources::strategy::GID_COMPACT_LEGEND;
const LEGEND_BUTTON_SIZE: f32 = 30.0;

fn legend_button_origin(faction: CockpitFaction) -> egui::Pos2 {
    match faction {
        CockpitFaction::Alliance => egui::pos2(55.0, 50.0),
        CockpitFaction::Empire => egui::pos2(113.0, 50.0),
    }
}

/// The window's close button (`FUN_004522f0`): 0x277c, 0x277d pressed.
const CLOSE_BUTTON: u32 = 0x277c;
const CLOSE_BUTTON_PRESSED: u32 = 0x277d;

/// `FUN_00426d00` keeps a 180 x 240 window inside the command centre.
const CLAMP_SIZE: egui::Vec2 = egui::vec2(180.0, 240.0);

/// The legend layout a display shows (`FUN_004522f0`'s switch); Popular
/// Support has one per side (`+0x9c` 1, the Alliance, `0x27c7`).
#[must_use]
pub fn legend_resource(mode: GidMode, faction: CockpitFaction) -> Option<u16> {
    Some(match mode {
        GidMode::PopularSupport => match faction {
            CockpitFaction::Alliance => 0x27c7,
            CockpitFaction::Empire => 0x27c8,
        },
        GidMode::Uprisings => 0x2d50,
        GidMode::IdleFleets => 0x2d51,
        GidMode::FleetsEnRoute => 0x2d52,
        GidMode::IdlePersonnel => 0x2d53,
        GidMode::ActivePersonnel => 0x2d54,
        GidMode::AvailableEnergy => 0x27cc,
        GidMode::AvailableRawMaterial => 0x27cd,
        GidMode::Mines => 0x27ce,
        GidMode::Refineries => 0x27cf,
        GidMode::Shipyards => 0x27d1,
        GidMode::TrainingFacilities => 0x27d2,
        GidMode::ConstructionYards => 0x27d3,
        GidMode::IdleShipyards => 0x2d55,
        GidMode::IdleTrainingFacilities => 0x2d56,
        GidMode::IdleConstructionYards => 0x2d57,
        GidMode::Troopers => 0x27d4,
        GidMode::FighterSquadrons => 0x27d5,
        GidMode::DeathStarShields => 0x27d6,
        GidMode::PlanetaryShieldGenerators => 0x27d7,
        GidMode::PlanetaryDefenseBatteries => 0x27d8,
        // FUN_004522f0 has no layout for the briefing's displays: its
        // default returns before FUN_00452630, so the original's legend
        // window is empty. port: no legend.
        GidMode::DisplayOff
        | GidMode::OwnLoyalSystems
        | GidMode::OwnMilitaryControl
        | GidMode::EnemyLoyalSystems
        | GidMode::EnemyMilitaryControl
        | GidMode::UnexploredSystems
        | GidMode::BriefingFleets
        | GidMode::AllDefenses
        | GidMode::MarkedImperialSystem
        | GidMode::MarkedAllianceSystem => return None,
    })
}

/// The legend window and its button, in the command centre's 640 x 480
/// coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct GidLegendWindow {
    pub open: bool,
    pub button_shown: bool,
    pub position: egui::Pos2,
    /// The position a drag started from.
    drag_anchor: Option<egui::Pos2>,
}

impl Default for GidLegendWindow {
    /// The game opens on Popular Support with the button and no legend, as
    /// the original's reference capture 0119 shows. hyp: how it leaves the
    /// constructor's Display Off (`FUN_00421c70`, `+0xc8` = `0x80`) is not
    /// read. The kept position starts at (150, 120) (`FUN_00421c70`,
    /// `+0x11c`/`+0x120`).
    fn default() -> Self {
        Self {
            open: false,
            button_shown: true,
            position: egui::pos2(150.0, 120.0),
            drag_anchor: None,
        }
    }
}

impl GidLegendWindow {
    /// A display was chosen (`FUN_00426d00`): Display Off closes the legend
    /// and hides the button; any other display opens the legend at its kept
    /// position, moved in so a 180 x 240 window fits, and shows the button.
    pub fn show_display(&mut self, mode: GidMode) {
        self.drag_anchor = None;
        if mode == GidMode::DisplayOff {
            self.open = false;
            self.button_shown = false;
            return;
        }
        let limit = egui::pos2(
            STRATEGIC_LOGICAL_WIDTH - CLAMP_SIZE.x,
            STRATEGIC_LOGICAL_HEIGHT - CLAMP_SIZE.y,
        );
        self.position = self.position.min(limit);
        self.open = true;
        self.button_shown = true;
    }

    /// The legend's close button: the legend hides, the button shows.
    pub fn close(&mut self) {
        self.open = false;
        self.button_shown = true;
        self.drag_anchor = None;
    }

    /// A click on a system of the map (`FUN_00422ce0`, `WM_LBUTTONUP` with
    /// a hovered item `+0x25c`) posts `0x405` to close a visible legend
    /// before `FUN_00429ce0` opens the system; a closed legend stays as it
    /// is. Returns whether it closed the legend.
    pub fn close_for_map_click(&mut self) -> bool {
        let open = self.open;
        if open {
            self.close();
        }
        open
    }

    /// The legend button (command `0x134`): the legend shows, the button
    /// hides. hyp: the original shows nothing until a legend exists
    /// (`+0x124`); the port opens one, the button being on screen.
    pub fn restore(&mut self) {
        self.open = true;
        self.button_shown = false;
    }

    /// Whether the legend window or its button covers `pointer`, a screen
    /// point, so the galaxy under it takes no click.
    #[must_use]
    pub fn contains_screen_point(
        &self,
        layout: CockpitLayout,
        mode: GidMode,
        faction: CockpitFaction,
        pointer: (f32, f32),
    ) -> bool {
        let size = legend_resource(mode, faction).and_then(legend_size);
        self.covers(
            layout,
            mode,
            faction,
            size,
            egui::pos2(pointer.0, pointer.1),
        )
    }

    fn covers(
        &self,
        layout: CockpitLayout,
        mode: GidMode,
        faction: CockpitFaction,
        size: Option<egui::Vec2>,
        pointer: egui::Pos2,
    ) -> bool {
        if mode == GidMode::DisplayOff {
            return false;
        }
        let logical = egui::pos2(
            (pointer.x - layout.canvas.x) / layout.scale,
            (pointer.y - layout.canvas.y) / layout.scale,
        );
        let button = egui::Rect::from_min_size(
            legend_button_origin(faction),
            egui::Vec2::splat(LEGEND_BUTTON_SIZE),
        );
        let window = size.map(|size| egui::Rect::from_min_size(self.position, size));
        (self.button_shown && button.contains(logical))
            || (self.open && window.is_some_and(|window| window.contains(logical)))
    }

    /// Follow a drag `total` logical pixels from where it started; `None`
    /// drops it. The window moves live (`FUN_006071e0`, `+0xf4` bit 0, set
    /// by `FUN_00606380`) and may leave the command centre while held; the
    /// drop (`FUN_00607010`) moves it back inside the bounds `FUN_006071a0`
    /// took from the command centre: `left` when it is left of them, else
    /// `right - width` when it passes them, and the same for the height.
    fn drag(&mut self, total: Option<egui::Vec2>, size: egui::Vec2) {
        let Some(total) = total else {
            if self.drag_anchor.take().is_some() {
                let inside = |at: f32, length: f32, bound: f32| {
                    if at < 0.0 {
                        0.0
                    } else if at + length > bound {
                        bound - length
                    } else {
                        at
                    }
                };
                self.position = egui::pos2(
                    inside(self.position.x, size.x, STRATEGIC_LOGICAL_WIDTH),
                    inside(self.position.y, size.y, STRATEGIC_LOGICAL_HEIGHT),
                );
            }
            return;
        };
        self.position = *self.drag_anchor.get_or_insert(self.position) + total;
    }
}

/// The frame tiles of `FUN_00607740(0x2779, 0x277a, 0x277b, 0x2778, 0x2775,
/// 0x2774, 0x2777, 0x2776)`: top, bottom, left and right edges, then the
/// top-left, top-right, bottom-left and bottom-right corners. Each tile is
/// grey outside and white inside (STRATEGY 10100..10107).
const FRAME_TOP: u32 = 0x2779;
const FRAME_BOTTOM: u32 = 0x277a;
const FRAME_LEFT: u32 = 0x277b;
const FRAME_RIGHT: u32 = 0x2778;
const FRAME_CORNERS: [u32; 4] = [0x2775, 0x2774, 0x2777, 0x2776];

/// One frame tile: its resource and its logical rectangle in a window of
/// `size`.
#[must_use]
pub fn frame_tiles(size: egui::Vec2) -> Vec<(u32, egui::Rect)> {
    let corner = egui::vec2(2.0, 2.0);
    let rect = |x: f32, y: f32, w: f32, h: f32| {
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    };
    let mut tiles = vec![
        (FRAME_CORNERS[0], rect(0.0, 0.0, corner.x, corner.y)),
        (
            FRAME_CORNERS[1],
            rect(size.x - corner.x, 0.0, corner.x, corner.y),
        ),
        (
            FRAME_CORNERS[2],
            rect(0.0, size.y - corner.y, corner.x, corner.y),
        ),
        (
            FRAME_CORNERS[3],
            rect(size.x - corner.x, size.y - corner.y, corner.x, corner.y),
        ),
    ];
    let inner = size - 2.0 * corner;
    if inner.x > 0.0 {
        tiles.push((FRAME_TOP, rect(corner.x, 0.0, inner.x, 2.0)));
        tiles.push((FRAME_BOTTOM, rect(corner.x, size.y - 2.0, inner.x, 2.0)));
    }
    if inner.y > 0.0 {
        tiles.push((FRAME_LEFT, rect(0.0, corner.y, 2.0, inner.y)));
        tiles.push((FRAME_RIGHT, rect(size.x - 2.0, corner.y, 2.0, inner.y)));
    }
    tiles
}

/// The close button's logical rectangle: `width - button width - 3`, 3.
#[must_use]
pub fn close_button_rect(width: f32, button: egui::Vec2) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(width - button.x - 3.0, 3.0), button)
}

/// Where a text item starts, from the window's top left: `y` 1 centres it
/// across the window's top (`FUN_00452630`), otherwise it starts at `x, y`.
#[must_use]
pub fn text_anchor(x: u16, y: u16, width: f32) -> (egui::Vec2, egui::Align2) {
    if y == 1 {
        (egui::vec2(width / 2.0, 1.0), egui::Align2::CENTER_TOP)
    } else {
        (
            egui::vec2(f32::from(x), f32::from(y)),
            egui::Align2::LEFT_TOP,
        )
    }
}

/// A command-centre point on the screen.
#[must_use]
pub fn to_screen(layout: CockpitLayout, logical: egui::Pos2) -> egui::Pos2 {
    egui::pos2(
        layout.canvas.x + logical.x * layout.scale,
        layout.canvas.y + logical.y * layout.scale,
    )
}

/// A command-centre rectangle on the screen.
#[must_use]
pub fn rect_to_screen(layout: CockpitLayout, logical: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        to_screen(layout, logical.min),
        logical.size() * layout.scale,
    )
}

/// Draw the legend button and the legend window for `mode`.
#[expect(
    clippy::cast_precision_loss,
    reason = "Bitmap sizes and legend offsets are small integers drawn as pixel coordinates."
)]
pub fn draw_gid_legend(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    window: &mut GidLegendWindow,
    mode: GidMode,
    faction: CockpitFaction,
    input_enabled: bool,
) {
    let scale = layout.scale;
    let full_uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));

    if window.button_shown && mode != GidMode::DisplayOff {
        let rect = rect_to_screen(
            layout,
            egui::Rect::from_min_size(
                legend_button_origin(faction),
                egui::Vec2::splat(LEGEND_BUTTON_SIZE),
            ),
        );
        let origin = rect.min;
        let clicked = egui::Area::new(egui::Id::new("gid_legend_button"))
            .order(egui::Order::Middle)
            .fade_in(false)
            .fixed_pos(origin)
            .show(ctx, |ui| {
                if !input_enabled {
                    ui.disable();
                }
                let response = ui.allocate_rect(rect, egui::Sense::click());
                // FUN_004286b0: the button's tooltip, TEXTSTRA 0x173c "Legend".
                if input_enabled {
                    if let Some(text) = crate::cockpit::control_tooltip_string(0x134)
                        .and_then(crate::win_tooltip::string)
                    {
                        crate::win_tooltip::attach(&response, &text);
                    }
                }
                if let Some(texture) = cache.get(ctx, DllSource::Strategy, LEGEND_BUTTON) {
                    let size = texture.size_vec2() * scale;
                    ui.painter_at(rect).image(
                        texture.id(),
                        egui::Rect::from_min_size(origin, size),
                        full_uv,
                        egui::Color32::WHITE,
                    );
                }
                response.clicked()
            })
            .inner;
        if clicked {
            window.restore();
        }
    }

    if !window.open {
        return;
    }
    let Some(legend) = legend_resource(mode, faction).and_then(legend) else {
        return;
    };
    let size = egui::vec2(f32::from(legend.width), f32::from(legend.height));
    let screen = rect_to_screen(layout, egui::Rect::from_min_size(window.position, size));
    let origin = screen.min;
    let close_size = cache
        .original_resource_size(DllSource::Strategy, CLOSE_BUTTON)
        .map_or(egui::vec2(14.0, 14.0), |[w, h]| {
            egui::vec2(w as f32, h as f32)
        });
    let close_logical = close_button_rect(size.x, close_size);
    let close = rect_to_screen(layout, close_logical.translate(window.position.to_vec2()));

    let (drag, closed) = egui::Area::new(egui::Id::new("gid_legend_window"))
        .order(egui::Order::Middle)
        .fade_in(false)
        .fixed_pos(origin)
        .show(ctx, |ui| {
            if !input_enabled {
                ui.disable();
            }
            let body = ui.allocate_rect(screen, egui::Sense::click_and_drag());
            let close_response = ui.interact(
                close,
                egui::Id::new("gid_legend_close"),
                egui::Sense::click(),
            );
            // A window dragged past the command centre is cut at its edge.
            let canvas = egui::Rect::from_min_size(
                egui::pos2(layout.canvas.x, layout.canvas.y),
                egui::vec2(layout.canvas.width, layout.canvas.height),
            );
            let painter = ui.painter_at(screen.intersect(canvas));
            // Each item at its offset from the window's top left.
            let at = |offset: egui::Vec2| to_screen(layout, window.position + offset);
            let mut image = |id: u32, offset: egui::Vec2, logical: Option<egui::Vec2>| {
                let Some(texture) = cache.get(ctx, DllSource::Strategy, id) else {
                    return;
                };
                let size = logical.unwrap_or_else(|| texture.size_vec2());
                painter.image(
                    texture.id(),
                    rect_to_screen(
                        layout,
                        egui::Rect::from_min_size(window.position + offset, size),
                    ),
                    full_uv,
                    egui::Color32::WHITE,
                );
            };
            for item in &legend.items {
                match item {
                    GidLegendItem::Bitmap { x, y, id } => image(
                        u32::from(*id),
                        egui::vec2(f32::from(*x), f32::from(*y)),
                        None,
                    ),
                    GidLegendItem::Text {
                        x,
                        y,
                        text,
                        font,
                        rgb,
                    } => {
                        let font_id = crate::theme::original_font(*font, scale);
                        let colour = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (offset, align) = text_anchor(*x, *y, size.x);
                        painter.text(at(offset), align, text, font_id, colour);
                    }
                }
            }
            for (id, tile) in frame_tiles(size) {
                image(id, tile.min.to_vec2(), Some(tile.size()));
            }
            let pressed = close_response.is_pointer_button_down_on();
            image(
                if pressed {
                    CLOSE_BUTTON_PRESSED
                } else {
                    CLOSE_BUTTON
                },
                close_logical.min.to_vec2(),
                None,
            );
            // From the press, since egui starts a drag past a threshold.
            let drag = (body.dragged() && !close_response.is_pointer_button_down_on())
                .then(|| {
                    ui.input(|input| {
                        let pointer = &input.pointer;
                        pointer.press_origin().zip(pointer.interact_pos())
                    })
                })
                .flatten()
                .map(|(from, to)| (to - from) / scale.max(f32::EPSILON));
            (drag, close_response.clicked())
        })
        .inner;
    if closed {
        window.close();
    } else {
        window.drag(drag, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legend_button_shows_its_tooltip_while_input_is_on() {
        // FUN_004286b0: the button (command 0x134) has TEXTSTRA 0x173c.
        crate::win_tooltip::set_strings(
            crate::win_tooltip::tests::TEST_STRINGS
                .iter()
                .map(|(id, text)| (*id, (*text).to_owned()))
                .collect(),
        );
        let faction = CockpitFaction::Empire;
        let layout = crate::cockpit::CockpitState::new(faction).layout_for(1280.0, 960.0);
        let origin = egui::pos2(layout.canvas.x, layout.canvas.y);
        let tip = |enabled: bool| {
            let mut cache = BmpCache::new();
            let mut window = GidLegendWindow::default();
            let pointer = origin + egui::vec2(120.0, 57.0) * layout.scale;
            let shapes = crate::win_tooltip::tests::frames(
                &egui::Context::default(),
                pointer,
                &[0.0, 0.1, 0.2, 0.3, 0.9],
                origin,
                layout.scale,
                |ctx| {
                    draw_gid_legend(
                        ctx,
                        &mut cache,
                        layout,
                        &mut window,
                        GidMode::PopularSupport,
                        faction,
                        enabled,
                    );
                },
            );
            crate::win_tooltip::tests::tip_shapes(&shapes).map(|(_, _, text)| text)
        };
        assert_eq!(tip(true).as_deref(), Some("Legend"));
        assert_eq!(tip(false), None);
    }

    #[test]
    fn each_display_shows_its_own_legend_layout() {
        // FUN_004522f0: Popular Support by side, the rest one each.
        assert_eq!(
            legend_resource(GidMode::PopularSupport, CockpitFaction::Alliance),
            Some(0x27c7)
        );
        assert_eq!(
            legend_resource(GidMode::PopularSupport, CockpitFaction::Empire),
            Some(0x27c8)
        );
        assert_eq!(
            legend_resource(GidMode::IdleShipyards, CockpitFaction::Empire),
            Some(0x2d55)
        );
        assert_eq!(
            legend_resource(GidMode::PlanetaryDefenseBatteries, CockpitFaction::Alliance),
            Some(0x27d8)
        );
        assert_eq!(
            legend_resource(GidMode::DisplayOff, CockpitFaction::Alliance),
            None
        );
    }

    #[test]
    fn choosing_a_display_opens_the_legend_inside_the_command_centre() {
        // FUN_00426d00: a display opens the legend and shows the button; the
        // kept position moves in so 180 x 240 fits in 640 x 480.
        let mut window = GidLegendWindow::default();
        assert!(!window.open && window.button_shown);
        assert_eq!(window.position, egui::pos2(150.0, 120.0));
        window.position = egui::pos2(500.0, 300.0);
        window.show_display(GidMode::Mines);
        assert!(window.open && window.button_shown);
        assert_eq!(window.position, egui::pos2(460.0, 240.0));
        // Display Off closes both.
        window.show_display(GidMode::DisplayOff);
        assert!(!window.open && !window.button_shown);
    }

    #[test]
    fn the_close_button_leaves_the_legend_button_which_restores_it() {
        // FUN_00422ce0: 0x405 case 3 hides the legend and shows the button;
        // command 0x134 shows the legend and hides the button.
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Uprisings);
        window.close();
        assert!(!window.open && window.button_shown);
        window.restore();
        assert!(window.open && !window.button_shown);
    }

    #[test]
    fn the_galaxy_takes_no_click_under_the_legend_or_its_button() {
        let layout = layout();
        let size = Some(egui::vec2(180.0, 135.0));
        let at = |x: f32, y: f32| egui::pos2(10.0 + x * 2.0, 20.0 + y * 2.0);
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        let covers = |window: &GidLegendWindow, mode, point| {
            window.covers(layout, mode, CockpitFaction::Alliance, size, point)
        };
        assert!(covers(&window, GidMode::Mines, at(150.0, 120.0)));
        assert!(covers(&window, GidMode::Mines, at(329.0, 254.0)));
        assert!(!covers(&window, GidMode::Mines, at(331.0, 200.0)));
        assert!(!covers(&window, GidMode::Mines, at(200.0, 256.0)));
        // The button, (55, 50) and 30 x 30 for the Alliance.
        assert!(covers(&window, GidMode::Mines, at(60.0, 60.0)));
        assert!(!covers(&window, GidMode::Mines, at(86.0, 60.0)));
        window.close();
        assert!(!covers(&window, GidMode::Mines, at(200.0, 200.0)));
        assert!(covers(&window, GidMode::Mines, at(60.0, 60.0)));
        window.restore();
        assert!(!covers(&window, GidMode::Mines, at(60.0, 60.0)));
        assert!(covers(&window, GidMode::Mines, at(200.0, 200.0)));
        // Display Off shows neither, nor a legend without its layout.
        assert!(!covers(&window, GidMode::DisplayOff, at(200.0, 200.0)));
        assert!(!window.covers(
            layout,
            GidMode::Mines,
            CockpitFaction::Alliance,
            None,
            at(200.0, 200.0)
        ));
    }

    #[test]
    fn a_click_on_a_system_closes_only_a_visible_legend() {
        // FUN_00422ce0: WM_LBUTTONUP on a hovered item posts 0x405 when the
        // legend is visible.
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        assert!(window.close_for_map_click());
        assert!(!window.open && window.button_shown);
        assert!(!window.close_for_map_click());
        // Display Off: no legend, no button; the click changes nothing.
        window.show_display(GidMode::DisplayOff);
        assert!(!window.close_for_map_click());
        assert!(!window.open && !window.button_shown);
    }

    #[test]
    fn a_dropped_legend_comes_back_inside_the_command_centre() {
        // FUN_006071e0 moves the window live, unbounded; FUN_00607010 moves
        // it back inside its parent's bounds on the drop.
        let mut window = GidLegendWindow::default();
        let size = egui::vec2(180.0, 135.0);
        window.drag(Some(egui::vec2(20.0, -10.0)), size);
        assert_eq!(window.position, egui::pos2(170.0, 110.0));
        // The drag counts from where it started, and may leave the centre.
        window.drag(Some(egui::vec2(-500.0, 900.0)), size);
        assert_eq!(window.position, egui::pos2(-350.0, 1020.0));
        window.drag(None, size);
        assert_eq!(window.position, egui::pos2(0.0, 345.0));
        window.drag(Some(egui::vec2(900.0, -900.0)), size);
        assert_eq!(window.position, egui::pos2(900.0, -555.0));
        window.drag(None, size);
        assert_eq!(window.position, egui::pos2(460.0, 0.0));
        // A drop a little past the right and bottom edges.
        window.drag(Some(egui::vec2(40.0, 400.0)), size);
        window.drag(None, size);
        assert_eq!(window.position, egui::pos2(460.0, 345.0));
        window.drag(Some(egui::vec2(-60.0, -305.0)), size);
        window.drag(None, size);
        assert_eq!(window.position, egui::pos2(400.0, 40.0));
        // A drop inside stays; no drag, no move.
        window.drag(None, size);
        assert_eq!(window.position, egui::pos2(400.0, 40.0));
    }

    #[test]
    fn the_frame_is_two_pixels_of_tiles_around_the_window() {
        // FUN_00607740: corners, then the edges tiled between them.
        let tiles = frame_tiles(egui::vec2(180.0, 135.0));
        let at = |id| {
            tiles
                .iter()
                .find(|(tile, _)| *tile == id)
                .map(|(_, rect)| *rect)
                .unwrap()
        };
        assert_eq!(
            at(0x2775),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(2.0, 2.0))
        );
        assert_eq!(at(0x2776).min, egui::pos2(178.0, 133.0));
        assert_eq!(
            at(0x2779),
            egui::Rect::from_min_size(egui::pos2(2.0, 0.0), egui::vec2(176.0, 2.0))
        );
        assert_eq!(at(0x2778).min, egui::pos2(178.0, 2.0));
        assert_eq!(at(0x277a).min, egui::pos2(2.0, 133.0));
        assert_eq!(at(0x277b).size(), egui::vec2(2.0, 131.0));
        assert_eq!(tiles.len(), 8);
    }

    #[test]
    fn the_close_button_sits_three_pixels_in_from_the_top_right() {
        // FUN_004522f0: x = width - button width - 3, y = 3.
        assert_eq!(
            close_button_rect(180.0, egui::vec2(14.0, 14.0)),
            egui::Rect::from_min_size(egui::pos2(163.0, 3.0), egui::vec2(14.0, 14.0))
        );
    }

    #[test]
    fn a_title_on_the_first_row_is_centred() {
        // FUN_00452630: y 1 centres the text across the window's top.
        assert_eq!(
            text_anchor(21, 1, 180.0),
            (egui::vec2(90.0, 1.0), egui::Align2::CENTER_TOP)
        );
        assert_eq!(
            text_anchor(25, 20, 180.0),
            (egui::vec2(25.0, 20.0), egui::Align2::LEFT_TOP)
        );
    }

    fn layout() -> CockpitLayout {
        let viewport = |x, y, width, height| crate::cockpit::CockpitViewport {
            x,
            y,
            width,
            height,
        };
        CockpitLayout {
            canvas: viewport(10.0, 20.0, 1280.0, 960.0),
            galaxy: viewport(0.0, 0.0, 0.0, 0.0),
            scale: 2.0,
        }
    }

    #[test]
    fn command_centre_points_scale_into_the_canvas() {
        assert_eq!(
            to_screen(layout(), egui::pos2(150.0, 120.0)),
            egui::pos2(310.0, 260.0)
        );
        assert_eq!(
            rect_to_screen(
                layout(),
                egui::Rect::from_min_size(egui::pos2(1.0, 2.0), egui::vec2(3.0, 4.0))
            ),
            egui::Rect::from_min_size(egui::pos2(12.0, 24.0), egui::vec2(6.0, 8.0))
        );
    }

    #[test]
    fn the_installed_layout_sizes_the_area_the_galaxy_gives_up() {
        set_gid_legends(GidLegends::from([(
            0x27ce,
            GidLegend {
                width: 180,
                height: 95,
                items: Vec::new(),
            },
        )]));
        assert_eq!(legend(0x27ce).map(|legend| legend.height), Some(95));
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        let contains = |window: &GidLegendWindow, mode, x: f32, y: f32| {
            window.contains_screen_point(
                layout(),
                mode,
                CockpitFaction::Alliance,
                (10.0 + x * 2.0, 20.0 + y * 2.0),
            )
        };
        assert!(contains(&window, GidMode::Mines, 200.0, 214.0));
        assert!(!contains(&window, GidMode::Mines, 200.0, 216.0));
        // A display without an installed layout leaves only the button.
        assert!(!contains(&window, GidMode::Refineries, 200.0, 200.0));
        assert!(contains(&window, GidMode::Refineries, 60.0, 60.0));
    }
}
