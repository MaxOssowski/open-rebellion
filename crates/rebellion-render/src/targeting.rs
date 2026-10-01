//! The galaxy view's targeting mode for a Mission order. Recovery notes:
//! `ghidra/notes/object-popup-menu.md`, "Targeting".
//!
//! Choosing Mission from the object pop-up menu hands the order to the galaxy
//! view (`FUN_00429320`): mode `+0xc0` becomes 2, the order waits at `+0xc4`,
//! the mouse is captured and the cursor is REBEXE.EXE cursor 1002. The next
//! left-button release (`FUN_00422ce0`) targets the object under the point.

use egui_macroquad::egui;
use rebellion_core::ids::SystemKey;
use rebellion_core::missions::MissionMember;

use crate::bmp_cache::{resources::rebexe, BmpCache, DllSource};
use crate::cockpit::CockpitLayout;

/// A Mission order waiting for its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targeting {
    team: Vec<MissionMember>,
}

/// How a release ends targeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetingEnd {
    /// The order's target is set (vtable `+0x2c`); `FUN_0042a320` opens the
    /// mission dialog, or hands the order back when no kind is legal.
    Target {
        team: Vec<MissionMember>,
        system: SystemKey,
    },
    /// No object was under the point, so the order is destroyed.
    Dropped,
}

impl Targeting {
    /// `FUN_00487c50` builds the order with the menu's selection as its team.
    #[must_use]
    pub fn new(team: Vec<MissionMember>) -> Self {
        Self { team }
    }

    #[must_use]
    pub fn team(&self) -> &[MissionMember] {
        &self.team
    }

    /// `FUN_00422ce0`'s `WM_LBUTTONUP` in mode 2, given the system the map's
    /// hit test found under the point.
    ///
    /// port: only a galaxy-map system is a target. The original also asks the
    /// window under the point (vtable `+0x68`) for a character or object and,
    /// for one of the team's own members, walks up to its system; system and
    /// sector windows are not hit-tested, so a release over them drops the
    /// order. Shift, which passes the click through to the window, is not
    /// ported either.
    #[must_use]
    pub fn release(self, system: Option<SystemKey>) -> TargetingEnd {
        match system {
            Some(system) => TargetingEnd::Target {
                team: self.team,
                system,
            },
            None => TargetingEnd::Dropped,
        }
    }
}

/// The screen rectangle of a cursor of `size` canvas pixels whose hotspot is
/// at `pointer`. The original's 640 by 480 screen scales with the canvas, so
/// the cursor does too.
#[must_use]
pub fn targeting_cursor_rect(
    layout: CockpitLayout,
    pointer: egui::Pos2,
    size: egui::Vec2,
) -> egui::Rect {
    let (hot_x, hot_y) = rebexe::TARGETING_CURSOR_HOTSPOT;
    let scale = layout.scale;
    egui::Rect::from_min_size(pointer - egui::vec2(hot_x, hot_y) * scale, size * scale)
}

/// Hold the pointer for the galaxy view while targeting. `FUN_00429320`
/// captures the mouse, so no other window receives a press or release until
/// targeting ends; the map reads its release through macroquad, beneath egui.
pub fn capture_pointer(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("original_targeting_capture"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.allocate_response(screen.size(), egui::Sense::click_and_drag());
        });
}

/// Draw the targeting cursor over every window at the pointer. egui paints a
/// painter-only layer after the areas of its order, so it also covers the
/// mission dialog, which shares Tooltip. Returns
/// false when cursor 1002 is not staged, so the caller keeps the system
/// cursor visible.
#[must_use]
pub fn draw_targeting_cursor(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    pointer: egui::Pos2,
) -> bool {
    let Some(texture) = cache.get(ctx, DllSource::Rebexe, rebexe::TARGETING_CURSOR) else {
        return false;
    };
    let rect = targeting_cursor_rect(layout, pointer, texture.size_vec2());
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("original_targeting_cursor"),
    ));
    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::CockpitViewport;
    use rebellion_core::ids::CharacterKey;

    fn layout(scale: f32) -> CockpitLayout {
        let viewport = CockpitViewport {
            x: 10.0,
            y: 20.0,
            width: 640.0 * scale,
            height: 480.0 * scale,
        };
        CockpitLayout {
            canvas: viewport,
            galaxy: viewport,
            scale,
        }
    }

    fn team() -> Vec<MissionMember> {
        vec![MissionMember::Character(CharacterKey::default())]
    }

    #[test]
    fn a_release_on_a_system_targets_it_with_the_team() {
        // FUN_00422ce0 WM_LBUTTONUP, mode 2: the target is set (+0x2c).
        let system = SystemKey::default();
        let targeting = Targeting::new(team());
        assert_eq!(targeting.team(), team().as_slice());

        assert_eq!(
            targeting.release(Some(system)),
            TargetingEnd::Target {
                team: team(),
                system,
            }
        );
    }

    #[test]
    fn a_release_on_no_object_drops_the_order() {
        // FUN_00422ce0: with no object under the point the order is destroyed.
        assert_eq!(Targeting::new(team()).release(None), TargetingEnd::Dropped);
    }

    #[test]
    fn the_cursor_hotspot_sits_on_the_pointer_at_every_scale() {
        // REBEXE.EXE cursor 1002: 32 by 32, hotspot (12, 12).
        let size = egui::vec2(32.0, 32.0);
        let pointer = egui::pos2(300.0, 200.0);

        let at_1 = targeting_cursor_rect(layout(1.0), pointer, size);
        assert_eq!(at_1.min, egui::pos2(288.0, 188.0));
        assert_eq!(at_1.size(), size);

        let at_2 = targeting_cursor_rect(layout(2.0), pointer, size);
        assert_eq!(at_2.min, egui::pos2(276.0, 176.0));
        assert_eq!(at_2.size(), egui::vec2(64.0, 64.0));
    }

    #[test]
    fn the_staged_cursor_is_drawn_on_top_with_its_hotspot_on_the_pointer() {
        // REBEXE.EXE cursor 1002, staged by extract-dll-resources.py --cursors.
        let root = std::env::temp_dir().join(format!("targeting-cursor-{}", std::process::id()));
        let dir = root.join("rebexe-exe/BMP");
        std::fs::create_dir_all(&dir).unwrap();
        let (side, stride, offset) = (32usize, 96usize, 54usize);
        let mut bmp = vec![0xff_u8; offset + stride * side];
        bmp[..2].copy_from_slice(b"BM");
        bmp[10..14].copy_from_slice(&(offset as u32).to_le_bytes());
        bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&(side as i32).to_le_bytes());
        bmp[22..26].copy_from_slice(&(side as i32).to_le_bytes());
        bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
        bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
        bmp[30..54].fill(0);
        std::fs::write(dir.join("1002.bmp"), bmp).unwrap();
        let mut cache = BmpCache::new();
        cache.set_base_path(&root);

        let ctx = egui::Context::default();
        // A new Area is laid out unseen on its first frame; draw two.
        let mut output = None;
        let mut drawn = false;
        for _ in 0..2 {
            output = Some(ctx.run(egui::RawInput::default(), |ctx| {
                drawn =
                    draw_targeting_cursor(ctx, &mut cache, layout(2.0), egui::pos2(100.0, 100.0));
                // The mission dialog's order, drawn after the cursor.
                egui::Area::new(egui::Id::new("window_below"))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        ui.label("a window");
                    });
            }));
        }
        let output = output.unwrap();
        assert!(drawn);
        assert!(
            output
                .shapes
                .iter()
                .any(|clipped| matches!(clipped.shape, egui::Shape::Text(_))),
            "the window below is painted"
        );
        let texture = cache
            .get(&ctx, DllSource::Rebexe, rebexe::TARGETING_CURSOR)
            .unwrap()
            .id();
        std::fs::remove_dir_all(root).unwrap();

        let last = output.shapes.last().expect("the cursor is drawn");
        let egui::Shape::Mesh(mesh) = &last.shape else {
            panic!("the topmost shape is not the cursor: {:?}", last.shape);
        };
        assert_eq!(mesh.texture_id, texture);
        assert_eq!(
            mesh.calc_bounds(),
            egui::Rect::from_min_size(egui::pos2(76.0, 76.0), egui::vec2(64.0, 64.0))
        );
    }

    #[test]
    fn the_capture_keeps_a_click_from_the_window_below() {
        // FUN_00429320 captures the mouse for the galaxy view.
        let click = |capture: bool| {
            let ctx = egui::Context::default();
            let mut clicked = false;
            for frame in 0..4 {
                let pointer = egui::pos2(20.0, 20.0);
                let mut events = vec![egui::Event::PointerMoved(pointer)];
                if frame >= 2 {
                    events.push(egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 2,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(640.0, 480.0),
                    )),
                    events,
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    egui::Area::new(egui::Id::new("window_below"))
                        .fixed_pos(egui::Pos2::ZERO)
                        .show(ctx, |ui| {
                            let response = ui
                                .allocate_response(egui::vec2(100.0, 100.0), egui::Sense::click());
                            clicked |= response.clicked();
                        });
                    if capture {
                        capture_pointer(ctx);
                    }
                });
            }
            clicked
        };

        assert!(click(false), "the window takes the click without a capture");
        assert!(!click(true));
    }

    #[test]
    fn an_unstaged_cursor_draws_nothing_and_says_so() {
        let mut cache = BmpCache::new();
        let ctx = egui::Context::default();
        let mut drawn = true;
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            drawn = draw_targeting_cursor(ctx, &mut cache, layout(1.0), egui::pos2(10.0, 10.0));
        });

        assert!(!drawn);
        assert!(output.shapes.is_empty());
    }
}
