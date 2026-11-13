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

/// The bounds a dragged legend button is dropped inside (`FUN_00427270`
/// passes them to `FUN_00452ab0`, which keeps them at `+0x11c..+0x128`):
/// left, top, right, bottom (55, 40, 540, 390) for the Alliance and
/// (120, 40, 600, 395) for the Empire.
fn legend_button_bounds(faction: CockpitFaction) -> egui::Rect {
    match faction {
        CockpitFaction::Alliance => {
            egui::Rect::from_min_max(egui::pos2(55.0, 40.0), egui::pos2(540.0, 390.0))
        }
        CockpitFaction::Empire => {
            egui::Rect::from_min_max(egui::pos2(120.0, 40.0), egui::pos2(600.0, 395.0))
        }
    }
}

/// Where the dropped button's outline at `at` puts it (`FUN_00452b70` case
/// `0x202`): past the bounds' right edge it goes to `right - 30`, else left
/// of them to `left`; above them to `top`, else past their bottom to
/// `bottom - 30`.
fn inside_button_bounds(at: egui::Pos2, faction: CockpitFaction) -> egui::Pos2 {
    let bounds = legend_button_bounds(faction);
    let corner = at + egui::Vec2::splat(LEGEND_BUTTON_SIZE);
    let x = if bounds.max.x < corner.x {
        bounds.max.x - LEGEND_BUTTON_SIZE
    } else if at.x < bounds.min.x {
        bounds.min.x
    } else {
        at.x
    };
    let y = if at.y < bounds.min.y {
        bounds.min.y
    } else if bounds.max.y < corner.y {
        bounds.max.y - LEGEND_BUTTON_SIZE
    } else {
        at.y
    };
    egui::pos2(x, y)
}

/// A drag of the legend button (`FUN_00452b70`, `+0x118` set): the point
/// of the move that began it (`+0x13c`/`+0x140`) and the outline's place
/// (`+0x12c`/`+0x130`), drawn from the next move on.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ButtonDrag {
    anchor: egui::Pos2,
    outline: egui::Pos2,
    drawn: bool,
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
    /// The last press on the legend button that began a double click: its
    /// time and command-centre point ([`Self::press_button`]).
    button_press: Option<(f64, egui::Pos2)>,
    /// Where a drag left the legend button; `None` keeps `FUN_00427270`'s
    /// place for the side.
    button_position: Option<egui::Pos2>,
    button_drag: Option<ButtonDrag>,
    /// The galaxy view's starfield, markers and caption this frame, which
    /// the window shows shaded under its art (`FUN_00452240`).
    backdrop: u32,
    drawn_gid_markers: Vec<(u32, [f32; 2])>,
    gid_caption: Option<crate::GidCaption>,
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
            button_press: None,
            button_position: None,
            button_drag: None,
            backdrop: crate::bmp_cache::resources::strategy::GALAXY_STARFIELD_DIM,
            drawn_gid_markers: Vec::new(),
            gid_caption: None,
        }
    }
}

impl GidLegendWindow {
    /// A display was chosen (`FUN_00426d00`): Display Off closes the legend
    /// and hides the button; any other display shows the button and builds
    /// the legend at its kept position, moved in so a 180 x 240 window
    /// fits, but puts it at the bottom of the z-order (`SetWindowPos`
    /// `HWND_BOTTOM`), under the galaxy view: A0 shows only the button after
    /// every display chosen from the menu or by Alt+1..9 (`gid/d-*.png`,
    /// `gid-displays/alt-*.png`).
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
        self.open = false;
        self.button_shown = true;
    }

    /// Take what the galaxy view drew this frame for `mode`: its starfield
    /// (`crate::gid_backdrop_resource`), its markers
    /// (`GalaxyMapState::drawn_gid_markers`) and its caption
    /// ([`crate::gid_caption`]). `FUN_00452240` copies the view's picture
    /// under the window and shades it (`FUN_005fe050` with the table
    /// `FUN_004522f0` builds, `FUN_005fddd0(LAB_004ac550)`), as the sector
    /// window does.
    pub fn set_view(
        &mut self,
        mode: GidMode,
        drawn_gid_markers: &[(u32, [f32; 2])],
        gid_caption: Option<crate::GidCaption>,
    ) {
        self.backdrop = crate::gid_backdrop_resource(mode);
        self.drawn_gid_markers.clear();
        self.drawn_gid_markers.extend_from_slice(drawn_gid_markers);
        self.gid_caption = gid_caption;
    }

    /// A left press at `time` (seconds) and `point` (command-centre
    /// pixels), on the legend button when `on_button`; returns whether it
    /// sends the button's command (`0x134`). `FUN_00427270` builds the
    /// button with `FUN_00452ab0`'s flags 3 (`+0x114`), and its window
    /// procedure (`Class_00659e38_vt14`, `FUN_00452b70`) swallows
    /// `WM_LBUTTONDOWN` and `WM_LBUTTONUP` and posts `WM_COMMAND` on
    /// `WM_LBUTTONDBLCLK` (`0x203`) alone. Windows sends that for a second
    /// press on the same window within the double-click time and rectangle
    /// (`crate::system_window::is_double_click`); a press elsewhere, or the
    /// double click itself, starts over. A0 2026-11-13-gid/A1, A2: a press
    /// held and a click open nothing; A3: a double click opens the legend.
    pub fn press_button(&mut self, time: f64, point: egui::Pos2, on_button: bool) -> bool {
        if !on_button {
            self.button_press = None;
            return false;
        }
        let double = self.button_press.is_some_and(|(at, from)| {
            crate::system_window::is_double_click(time - at, point - from)
        });
        self.button_press = (!double).then_some((time, point));
        double
    }

    /// The legend button's place in command-centre pixels.
    #[must_use]
    pub fn button_origin(&self, faction: CockpitFaction) -> egui::Pos2 {
        self.button_position
            .unwrap_or_else(|| legend_button_origin(faction))
    }

    /// The pointer moved to `point` (command-centre pixels). `FUN_00452b70`
    /// case `0x200`, the button's flags bit 1 (`FUN_00452ab0`'s 3): with
    /// the left button `held` and the point on an opaque pixel of the
    /// button's art (`hit`, `FUN_005fca00`) a move starts a drag and
    /// captures the mouse; each later move puts the outline (an XOR
    /// rectangle) at the button's place moved by the pointer since that
    /// first move. The button itself stays until the release.
    pub fn move_button_pointer(
        &mut self,
        faction: CockpitFaction,
        point: egui::Pos2,
        held: bool,
        hit: bool,
    ) {
        let origin = self.button_origin(faction);
        match &mut self.button_drag {
            Some(drag) => {
                drag.outline = origin + (point - drag.anchor);
                drag.drawn = true;
            }
            None if held && hit => {
                self.button_drag = Some(ButtonDrag {
                    anchor: point,
                    outline: origin,
                    drawn: false,
                });
            }
            None => {}
        }
    }

    /// The left button came up (`FUN_00452b70` case `0x202`): a drag ends
    /// and the button moves to its outline, kept inside the side's bounds
    /// ([`inside_button_bounds`]).
    pub fn release_button(&mut self, faction: CockpitFaction) {
        if let Some(drag) = self.button_drag.take() {
            self.button_position = Some(inside_button_bounds(drag.outline, faction));
        }
    }

    /// The outline of a drag under way, once a move has drawn it.
    #[must_use]
    pub fn button_outline(&self) -> Option<egui::Pos2> {
        self.button_drag
            .filter(|drag| drag.drawn)
            .map(|drag| drag.outline)
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
    /// (`+0x124`); the port opens one, the button being on screen. Only a
    /// double click sends the command ([`Self::press_button`]).
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
        self.covers(layout, faction, size, egui::pos2(pointer.0, pointer.1))
    }

    fn covers(
        &self,
        layout: CockpitLayout,
        faction: CockpitFaction,
        size: Option<egui::Vec2>,
        pointer: egui::Pos2,
    ) -> bool {
        let logical = egui::pos2(
            (pointer.x - layout.canvas.x) / layout.scale,
            (pointer.y - layout.canvas.y) / layout.scale,
        );
        let button = egui::Rect::from_min_size(
            self.button_origin(faction),
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
    // The sides go on every second row from the third (the loop steps by
    // two rows; A0 sector-windows and the GID menus show the rows between
    // as the window's fill, `cockpit::frame_tiles`).
    let mut y = corner.y;
    while y < size.y - corner.y {
        tiles.push((FRAME_LEFT, rect(0.0, y, 2.0, 1.0)));
        tiles.push((FRAME_RIGHT, rect(size.x - 2.0, y, 2.0, 1.0)));
        y += 2.0;
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

/// The legend button's drag (`FUN_00452b70`, flags bit 1): the frame's
/// pointer moves, then the release, on the button at `button` (screen).
/// The button swallows its press (`WM_LBUTTONDOWN`, flags & 3), so no
/// window holds the mouse and the moves over it reach it while the button
/// pressed on it is held. While dragged its outline follows the pointer;
/// port: the outline is drawn white where the original XORs a white pen
/// (`SetROP2(R2_XORPEN)`) into the palette.
fn drag_button(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    window: &mut GidLegendWindow,
    faction: CockpitFaction,
    input_enabled: bool,
    button: egui::Rect,
) {
    let (moves, down, press) = ctx.input(|input| {
        let moves: Vec<egui::Pos2> = input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::PointerMoved(point) => Some(*point),
                _ => None,
            })
            .collect();
        (
            moves,
            input.pointer.primary_down(),
            input.pointer.press_origin(),
        )
    });
    let layer = Some(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("gid_legend_button"),
    ));
    let on_button = |point: egui::Pos2| {
        input_enabled && button.contains(point) && ctx.layer_id_at(point) == layer
    };
    let held = down && press.is_some_and(on_button);
    let logical = |point: egui::Pos2| {
        egui::pos2(
            (point.x - layout.canvas.x) / layout.scale,
            (point.y - layout.canvas.y) / layout.scale,
        )
    };
    for point in moves {
        let at = logical(point);
        let inside = at - window.button_origin(faction);
        // FUN_005fca00 on the button's art at the point's offset.
        let hit = on_button(point)
            && inside.x >= 0.0
            && inside.y >= 0.0
            && cache.is_resource_hit(
                DllSource::Strategy,
                LEGEND_BUTTON,
                inside.x as usize,
                inside.y as usize,
            );
        window.move_button_pointer(faction, at, held, hit);
    }
    if !down {
        window.release_button(faction);
    }
    if let Some(outline) = window.button_outline() {
        // GDI's Rectangle with a 1-pixel pen: the outline's outer pixels.
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("gid_legend_button_outline"),
        ));
        let side = LEGEND_BUTTON_SIZE;
        for (x, y, w, h) in [
            (0.0, 0.0, side, 1.0),
            (0.0, side - 1.0, side, 1.0),
            (0.0, 1.0, 1.0, side - 2.0),
            (side - 1.0, 1.0, 1.0, side - 2.0),
        ] {
            painter.rect_filled(
                rect_to_screen(
                    layout,
                    egui::Rect::from_min_size(outline + egui::vec2(x, y), egui::vec2(w, h)),
                ),
                0.0,
                egui::Color32::WHITE,
            );
        }
    }
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

    // Only FUN_00426d00 hides the button (choosing Display Off); a new
    // game starts on Display Off with the button shown (A0
    // new-alliance/t18).
    if window.button_shown {
        let rect = rect_to_screen(
            layout,
            egui::Rect::from_min_size(
                window.button_origin(faction),
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
                response.rect
            })
            .inner;
        // Each left press anywhere, in order (two may come in one frame): on
        // the button it may complete a double click.
        let presses: Vec<(f64, egui::Pos2)> = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => Some((input.time, *pos)),
                    _ => None,
                })
                .collect()
        });
        for (time, point) in presses {
            let logical = egui::pos2(
                (point.x - layout.canvas.x) / scale,
                (point.y - layout.canvas.y) / scale,
            );
            let on_button = input_enabled
                && clicked.contains(point)
                && ctx.layer_id_at(point)
                    == Some(egui::LayerId::new(
                        egui::Order::Middle,
                        egui::Id::new("gid_legend_button"),
                    ));
            if window.press_button(time, logical, on_button) {
                window.restore();
                break;
            }
        }
        if window.button_shown {
            drag_button(ctx, cache, layout, window, faction, input_enabled, clicked);
        }
    } else {
        window.button_press = None;
        window.button_drag = None;
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
            // FUN_00452240: the galaxy view's picture under the window,
            // shaded, then the layout and the frame.
            crate::sector_window::paint_shaded_view(
                ctx,
                &painter,
                cache,
                layout,
                faction,
                screen,
                &crate::sector_window::ShadedView {
                    backdrop: window.backdrop,
                    drawn_gid_markers: &window.drawn_gid_markers,
                    gid_caption: window.gid_caption.as_ref(),
                },
            );
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
                        let colour = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (offset, align) = text_anchor(*x, *y, size.x);
                        // DrawTextA from the anchor's top: centred across
                        // the legend, or from its point.
                        let (left, gdi_align) = if align == egui::Align2::CENTER_TOP {
                            (0.0, crate::theme::GdiAlign::Center)
                        } else {
                            (offset.x, crate::theme::GdiAlign::Left)
                        };
                        let start = at(egui::vec2(left, offset.y));
                        crate::theme::paint_gdi_text(
                            &painter,
                            egui::Rect::from_min_size(
                                start,
                                egui::vec2((size.x - left) * scale, 30.0 * scale),
                            ),
                            scale,
                            *font,
                            text,
                            colour,
                            (gdi_align, crate::theme::GdiVAlign::Top),
                        );
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
    fn choosing_a_display_shows_the_button_and_keeps_the_legend_inside() {
        // FUN_00426d00: a display shows the button and builds the legend
        // under the galaxy view (A0 gid/d-*.png show the button alone); the
        // kept position moves in so 180 x 240 fits in 640 x 480.
        let mut window = GidLegendWindow::default();
        assert!(!window.open && window.button_shown);
        assert_eq!(window.position, egui::pos2(150.0, 120.0));
        window.position = egui::pos2(500.0, 300.0);
        window.show_display(GidMode::Mines);
        assert!(!window.open && window.button_shown);
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
        let covers = |window: &GidLegendWindow, point| {
            window.covers(layout, CockpitFaction::Alliance, size, point)
        };
        // The button, (55, 50) and 30 x 30 for the Alliance; the legend it
        // opens, 180 by 135 at (150, 120).
        assert!(covers(&window, at(60.0, 60.0)));
        assert!(!covers(&window, at(86.0, 60.0)));
        assert!(!covers(&window, at(150.0, 120.0)));
        window.restore();
        assert!(covers(&window, at(150.0, 120.0)));
        assert!(covers(&window, at(329.0, 254.0)));
        assert!(!covers(&window, at(331.0, 200.0)));
        assert!(!covers(&window, at(200.0, 256.0)));
        window.close();
        assert!(!covers(&window, at(200.0, 200.0)));
        assert!(covers(&window, at(60.0, 60.0)));
        window.restore();
        assert!(!covers(&window, at(60.0, 60.0)));
        assert!(covers(&window, at(200.0, 200.0)));
        // Display Off has no layout, so no legend; a legend without its
        // layout covers nothing. The button follows only `button_shown`
        // (A0 new-alliance/t18: a new game on Display Off shows it).
        assert!(legend_resource(GidMode::DisplayOff, CockpitFaction::Alliance).is_none());
        window.close();
        assert!(covers(&window, at(60.0, 60.0)));
        assert!(!window.covers(layout, CockpitFaction::Alliance, None, at(200.0, 200.0)));
    }

    #[test]
    fn a_click_on_a_system_closes_only_a_visible_legend() {
        // FUN_00422ce0: WM_LBUTTONUP on a hovered item posts 0x405 when the
        // legend is visible.
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        window.restore();
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
        // FUN_00607740: corners, then the edges tiled between them, the
        // sides on every second row (A0 sector-windows/02-second).
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
        // The sides on every second row: 2 by 1 at y 2, 4, .. 132.
        assert_eq!(
            at(0x277b),
            egui::Rect::from_min_size(egui::pos2(0.0, 2.0), egui::vec2(2.0, 1.0))
        );
        assert_eq!(tiles.iter().filter(|(id, _)| *id == 0x277b).count(), 66);
        assert_eq!(tiles.len(), 4 + 2 + 2 * 66);
        // An even height stops the sides two rows above the bottom corners.
        let even = frame_tiles(egui::vec2(180.0, 136.0));
        let last = even
            .iter()
            .filter(|(id, _)| *id == 0x277b)
            .map(|(_, rect)| rect.min.y)
            .fold(0.0, f32::max);
        assert_eq!(last, 132.0);
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
        window.restore();
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
        window.close();
        assert!(!contains(&window, GidMode::Refineries, 200.0, 200.0));
        assert!(contains(&window, GidMode::Refineries, 60.0, 60.0));
    }

    fn mines_legend() {
        set_gid_legends(GidLegends::from([(
            0x27ce,
            GidLegend {
                width: 180,
                height: 95,
                items: Vec::new(),
            },
        )]));
    }

    /// One frame of `draw_gid_legend` at `time` with `events`, its shapes.
    fn legend_frame(
        ctx: &egui::Context,
        cache: &mut BmpCache,
        window: &mut GidLegendWindow,
        time: f64,
        events: Vec<egui::Event>,
    ) -> Vec<egui::epaint::ClippedShape> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        };
        ctx.run(input, |ctx| {
            draw_gid_legend(
                ctx,
                cache,
                layout(),
                window,
                GidMode::Mines,
                CockpitFaction::Alliance,
                true,
            );
        })
        .shapes
    }

    #[test]
    fn the_legend_button_opens_the_legend_only_on_a_double_click() {
        // FUN_00427270 builds the button with FUN_00452ab0's flags 3;
        // Class_00659e38_vt14 swallows WM_LBUTTONDOWN and WM_LBUTTONUP and
        // posts command 0x134 on WM_LBUTTONDBLCLK. A0 2026-11-13-gid/A1,
        // A2: a press held and a click open nothing; A3: a double click
        // opens the legend.
        mines_legend();
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        // The button's middle, (70, 65), on the 2x canvas at (10, 20).
        let point = egui::pos2(10.0 + 70.0 * 2.0, 20.0 + 65.0 * 2.0);
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let mut time = 0.0;
        let mut frame = |window: &mut GidLegendWindow, events| {
            time += 0.05;
            let _ = legend_frame(&ctx, &mut cache, window, time, events);
        };
        frame(&mut window, vec![egui::Event::PointerMoved(point)]);
        // A press held a second, then released.
        frame(&mut window, vec![button(true)]);
        for _ in 0..20 {
            frame(&mut window, Vec::new());
        }
        frame(&mut window, vec![button(false)]);
        frame(&mut window, Vec::new());
        assert!(
            !window.open && window.button_shown,
            "a held press opened the legend"
        );
        for _ in 0..20 {
            frame(&mut window, Vec::new());
        }
        // A click.
        frame(&mut window, vec![button(true)]);
        frame(&mut window, vec![button(false)]);
        assert!(
            !window.open && window.button_shown,
            "a click opened the legend"
        );
        for _ in 0..20 {
            frame(&mut window, Vec::new());
        }
        // A click elsewhere, then at once a double click on the button: the
        // first press elsewhere pairs with nothing (WM_LBUTTONDBLCLK needs
        // both presses on the button's window).
        let away = egui::pos2(10.0 + 300.0 * 2.0, 20.0 + 300.0 * 2.0);
        let elsewhere = |pressed| egui::Event::PointerButton {
            pos: away,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frame(
            &mut window,
            vec![egui::Event::PointerMoved(away), elsewhere(true)],
        );
        frame(&mut window, vec![elsewhere(false)]);
        frame(&mut window, vec![egui::Event::PointerMoved(point)]);
        frame(&mut window, vec![button(true)]);
        assert!(!window.open, "a press elsewhere paired with the button's");
        frame(&mut window, vec![button(false)]);
        assert!(
            !window.open,
            "a click elsewhere and one on the button opened the legend"
        );
        // The second press of the double click opens it.
        frame(&mut window, vec![button(true)]);
        assert!(
            window.open && !window.button_shown,
            "a double click left the legend shut"
        );
        frame(&mut window, vec![button(false)]);
    }

    #[test]
    fn the_legend_takes_the_views_starfield_markers_and_caption() {
        // FUN_00452240 copies what the galaxy view drew: its starfield for
        // the display (902 with the display off, 903 otherwise), its marks.
        let mut window = GidLegendWindow::default();
        window.set_view(GidMode::DisplayOff, &[(10146, [1.0, 2.0])], None);
        assert_eq!(window.backdrop, 902);
        assert_eq!(window.drawn_gid_markers, [(10146, [1.0, 2.0])]);
        window.set_view(GidMode::Mines, &[], None);
        assert_eq!((window.backdrop, window.drawn_gid_markers.len()), (903, 0));
    }

    #[test]
    fn two_presses_on_the_button_pair_within_the_double_click_time_and_rectangle() {
        // WM_LBUTTONDBLCLK: the second press within GetDoubleClickTime
        // (500 ms) and 2 pixels; the pair starts over afterwards.
        let mut window = GidLegendWindow::default();
        let at = egui::pos2(70.0, 65.0);
        assert!(!window.press_button(0.0, at, true));
        assert!(window.press_button(0.4, at + egui::vec2(2.0, -2.0), true));
        // A third press begins a new pair.
        assert!(!window.press_button(0.5, at, true));
        assert!(!window.press_button(1.1, at, true));
        assert!(!window.press_button(1.2, at + egui::vec2(3.0, 0.0), true));
        // A press off the button breaks the pair.
        assert!(!window.press_button(1.3, at, false));
        assert!(!window.press_button(1.4, at, true));
        assert!(window.press_button(1.5, at, true));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_legend_shows_the_galaxy_view_shaded_under_it() {
        // FUN_00452240 copies the galaxy view's picture under the window
        // (FUN_005fcc30) and maps it through FUN_005fddd0(LAB_004ac550)'s
        // table (FUN_005fe050), as the sector window does. A0
        // 2026-11-13-gid/A3: the starfield under the legend shaded, (83,
        // 84, 128) as (69, 67, 68).
        mines_legend();
        let base = std::env::temp_dir().join(format!(
            "open-rebellion-legend-shade-{}",
            std::process::id()
        ));
        let folder = base.join("strategy-dll").join("BMP");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("903.bmp"),
            crate::bmp_cache::tests::indexed_bmp(&[[83, 84, 128], [69, 67, 68]]),
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        cache.set_base_path(&base);
        let shaded = cache
            .get_shaded(&ctx, DllSource::Strategy, 903)
            .map(egui::TextureHandle::id)
            .unwrap();
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        window.set_view(GidMode::Mines, &[], None);
        window.restore();
        // A new area shows from its second frame.
        let _ = legend_frame(&ctx, &mut cache, &mut window, 0.05, Vec::new());
        let shapes = legend_frame(&ctx, &mut cache, &mut window, 0.1, Vec::new());
        std::fs::remove_dir_all(base).unwrap();
        // The shaded starfield fills the legend's rectangle, (150, 120)
        // and 180 by 95, doubled on the canvas at (10, 20).
        let legend = egui::Rect::from_min_size(egui::pos2(310.0, 260.0), egui::vec2(360.0, 190.0));
        let painted = shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id == shaded => Some(
                mesh.vertices
                    .iter()
                    .fold(egui::Rect::NOTHING, |rect, vertex| {
                        rect.union(egui::Rect::from_min_max(vertex.pos, vertex.pos))
                    }),
            ),
            _ => None,
        });
        assert_eq!(painted, Some(legend));
    }

    /// A cache whose STRATEGY 10168 is a 30 by 30 bitmap opaque but for
    /// its key, the first stored pixel (bottom-left); and its folder.
    #[cfg(not(target_arch = "wasm32"))]
    fn button_art_cache() -> (BmpCache, std::path::PathBuf) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let base = std::env::temp_dir().join(format!(
            "open-rebellion-legend-button-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let folder = base.join("strategy-dll").join("BMP");
        std::fs::create_dir_all(&folder).unwrap();
        let (side, stride) = (30_u32, 32_u32);
        let offset = 14 + 40 + 4 * 2;
        let mut bytes = Vec::new();
        bytes.extend(b"BM");
        bytes.extend((offset + stride * side).to_le_bytes());
        bytes.extend([0; 4]);
        bytes.extend(offset.to_le_bytes());
        bytes.extend(40u32.to_le_bytes());
        bytes.extend(side.to_le_bytes());
        bytes.extend(side.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8u16.to_le_bytes());
        bytes.extend([0; 4]);
        bytes.extend((stride * side).to_le_bytes());
        bytes.extend([0; 8]);
        bytes.extend(2u32.to_le_bytes());
        bytes.extend([0; 4]);
        bytes.extend([0, 0, 0, 0, 255, 255, 255, 0]);
        for index in 0..stride * side {
            bytes.push(u8::from(index != 0));
        }
        std::fs::write(folder.join(format!("{LEGEND_BUTTON}.bmp")), bytes).unwrap();
        let mut cache = BmpCache::new();
        cache.set_base_path(&base);
        (cache, base)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_legend_button_drags_by_an_outline_and_drops_inside_its_bounds() {
        // FUN_00427270 builds the button with FUN_00452ab0's flags 3 and
        // the Alliance's bounds (55, 40, 540, 390). FUN_00452b70: with bit
        // 1, a move with the left button held on an opaque pixel
        // (FUN_005fca00) starts the drag; later moves put the XOR outline
        // at the button's place moved by the pointer since that move; the
        // release (0x202) moves the button there, kept inside the bounds.
        mines_legend();
        let ctx = egui::Context::default();
        let (mut cache, base) = button_art_cache();
        let mut window = GidLegendWindow::default();
        window.show_display(GidMode::Mines);
        let screen = |x: f32, y: f32| egui::pos2(10.0 + x * 2.0, 20.0 + y * 2.0);
        let button = |x, y, pressed| egui::Event::PointerButton {
            pos: screen(x, y),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let moved = |x, y| egui::Event::PointerMoved(screen(x, y));
        let mut time = 0.0;
        let mut frame = |window: &mut GidLegendWindow, events| {
            // Apart, so no two presses make a double click.
            time += 1.0;
            let _ = legend_frame(&ctx, &mut cache, window, time, events);
        };
        frame(&mut window, vec![moved(70.0, 65.0)]);
        frame(&mut window, vec![moved(70.0, 65.0)]);

        // A press, a move that starts the drag, a move that draws the
        // outline: the button stays until the release.
        frame(&mut window, vec![button(70.0, 65.0, true)]);
        frame(&mut window, vec![moved(71.0, 66.0)]);
        assert_eq!(window.button_outline(), None);
        frame(&mut window, vec![moved(171.0, 116.0)]);
        assert_eq!(window.button_outline(), Some(egui::pos2(155.0, 100.0)));
        assert_eq!(
            window.button_origin(CockpitFaction::Alliance),
            egui::pos2(55.0, 50.0)
        );
        frame(&mut window, vec![button(171.0, 116.0, false)]);
        let dropped = window.button_origin(CockpitFaction::Alliance);
        assert_eq!(
            (dropped, window.button_outline()),
            (egui::pos2(155.0, 100.0), None)
        );
        assert!(!window.open, "a drag opened the legend");
        let contains = |window: &GidLegendWindow, x: f32, y: f32| {
            window.contains_screen_point(
                layout(),
                GidMode::Mines,
                CockpitFaction::Alliance,
                (screen(x, y).x, screen(x, y).y),
            )
        };
        assert!(contains(&window, 170.0, 115.0) && !contains(&window, 70.0, 65.0));

        // Past the bounds' right and bottom: right - 30, bottom - 30.
        frame(&mut window, vec![moved(170.0, 115.0)]);
        frame(&mut window, vec![button(170.0, 115.0, true)]);
        frame(&mut window, vec![moved(171.0, 116.0)]);
        frame(&mut window, vec![moved(650.0, 460.0)]);
        frame(&mut window, vec![button(650.0, 460.0, false)]);
        assert_eq!(
            window.button_origin(CockpitFaction::Alliance),
            egui::pos2(510.0, 360.0)
        );

        // Left of and above them: left and top.
        frame(&mut window, vec![moved(525.0, 375.0)]);
        frame(&mut window, vec![button(525.0, 375.0, true)]);
        frame(&mut window, vec![moved(526.0, 376.0)]);
        frame(&mut window, vec![moved(1.0, 1.0)]);
        frame(&mut window, vec![button(1.0, 1.0, false)]);
        assert_eq!(
            window.button_origin(CockpitFaction::Alliance),
            egui::pos2(55.0, 40.0)
        );

        // A press off the button, or a move onto its edge column (offset
        // x 0, which FUN_005fca00 refuses), starts no drag.
        frame(&mut window, vec![moved(300.0, 300.0)]);
        frame(&mut window, vec![button(300.0, 300.0, true)]);
        frame(&mut window, vec![moved(70.0, 55.0)]);
        frame(&mut window, vec![moved(200.0, 200.0)]);
        frame(&mut window, vec![button(200.0, 200.0, false)]);
        frame(&mut window, vec![moved(70.0, 55.0)]);
        frame(&mut window, vec![button(70.0, 55.0, true)]);
        frame(&mut window, vec![moved(55.0, 55.0)]);
        frame(&mut window, vec![moved(200.0, 200.0)]);
        frame(&mut window, vec![button(200.0, 200.0, false)]);
        std::fs::remove_dir_all(base).unwrap();
        assert_eq!(
            window.button_origin(CockpitFaction::Alliance),
            egui::pos2(55.0, 40.0)
        );
    }

    #[test]
    fn the_empires_legend_button_drops_inside_its_own_bounds() {
        // FUN_00427270, the Empire's branch: bounds (120, 40, 600, 395).
        let mut window = GidLegendWindow::default();
        let empire = CockpitFaction::Empire;
        assert_eq!(window.button_origin(empire), egui::pos2(113.0, 50.0));
        window.move_button_pointer(empire, egui::pos2(120.0, 60.0), true, true);
        window.move_button_pointer(empire, egui::pos2(800.0, 600.0), true, true);
        window.release_button(empire);
        assert_eq!(window.button_origin(empire), egui::pos2(570.0, 365.0));
        window.move_button_pointer(empire, egui::pos2(580.0, 380.0), true, true);
        window.move_button_pointer(empire, egui::pos2(0.0, 0.0), true, true);
        window.release_button(empire);
        assert_eq!(window.button_origin(empire), egui::pos2(120.0, 40.0));
        // Inside them it stays where the outline was.
        window.move_button_pointer(empire, egui::pos2(130.0, 50.0), true, true);
        window.move_button_pointer(empire, egui::pos2(230.0, 150.0), true, true);
        window.release_button(empire);
        assert_eq!(window.button_origin(empire), egui::pos2(220.0, 140.0));
        // Without the button held, or off its art, a move starts nothing.
        window.move_button_pointer(empire, egui::pos2(230.0, 150.0), false, true);
        window.move_button_pointer(empire, egui::pos2(330.0, 250.0), true, false);
        assert_eq!(window.button_outline(), None);
        // The far corner decides: the outline's own corner (580, 370) lies
        // inside, the button's far one (610, 400) does not.
        window.move_button_pointer(empire, egui::pos2(230.0, 150.0), true, true);
        window.move_button_pointer(empire, egui::pos2(590.0, 380.0), true, true);
        window.release_button(empire);
        assert_eq!(window.button_origin(empire), egui::pos2(570.0, 365.0));
    }
}
