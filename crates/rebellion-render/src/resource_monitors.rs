//! The command center's resource monitors in the top status strip
//! (`ghidra/notes/side-resource-monitors.md`).
//!
//! `FUN_00422ce0` creates three 63 by 17 text objects and `FUN_00422620`
//! fills them through `FUN_00429200`.

use egui_macroquad::egui;
use rebellion_core::blockade::BlockadeState;
use rebellion_core::dat::Faction;
use rebellion_core::manufacturing::ManufacturingState;
use rebellion_core::production_cycle::ProductionState;
use rebellion_core::side_resources::{maintenance_totals, MONITOR_MAX};
use rebellion_core::uprising::UprisingState;
use rebellion_core::world::GameWorld;

use crate::cockpit::{logical_rect_to_screen, CockpitFaction, CockpitLayout, CockpitViewport};
use crate::game_menu::faction_text_color;

/// Game-font entry 8 (`FUN_0060eed0`): 12-pixel Arial, normal weight, drawn
/// with egui's proportional face. The day readout's 14-pixel entry 10 is
/// drawn at 12; the same ratio gives 12 * 12 / 14.
const MONITOR_FONT_HEIGHT: f32 = 10.3;

/// Gap between the text and the right edge of its rectangle.
const MONITOR_RIGHT_INSET: f32 = 1.0;

/// One of the three monitors, in `FUN_00429200` command order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceMonitor {
    /// Command `0x14c`, tooltip TEXTSTRA 5392.
    RawMaterials,
    /// Command `0x14d`, tooltip TEXTSTRA 5393.
    RefinedMaterials,
    /// Command `0x14e`, tooltip TEXTSTRA 5394.
    Maintenance,
}

/// A monitor's rectangle in the 640x480 canvas (`FUN_00422ce0`): 63 by 17 at
/// Alliance x 265, 360, 457 and y 16, Empire x 175, 273, 371 and y 19.
#[must_use]
pub const fn monitor_rect(faction: CockpitFaction, monitor: ResourceMonitor) -> CockpitViewport {
    let (x, y) = match (faction, monitor) {
        (CockpitFaction::Alliance, ResourceMonitor::RawMaterials) => (265.0, 16.0),
        (CockpitFaction::Alliance, ResourceMonitor::RefinedMaterials) => (360.0, 16.0),
        (CockpitFaction::Alliance, ResourceMonitor::Maintenance) => (457.0, 16.0),
        (CockpitFaction::Empire, ResourceMonitor::RawMaterials) => (175.0, 19.0),
        (CockpitFaction::Empire, ResourceMonitor::RefinedMaterials) => (273.0, 19.0),
        (CockpitFaction::Empire, ResourceMonitor::Maintenance) => (371.0, 19.0),
    };
    CockpitViewport {
        x,
        y,
        width: 63.0,
        height: 17.0,
    }
}

/// The value the cockpit's side reads on its Maintenance monitor. A system
/// in uprising or under blockade is blocked (`FUN_0050bbb0`).
#[must_use]
pub fn maintenance_available(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    blockade: &BlockadeState,
    uprising: &UprisingState,
    faction: CockpitFaction,
) -> i32 {
    let side = match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    };
    maintenance_totals(world, manufacturing, side, |system| {
        blockade.is_blockaded(system) || uprising.is_uprising(system)
    })
    .available()
}

/// The Maintenance monitor's text (`FUN_00429200`, command `0x14e`): the
/// number, or TEXTSTRA 4866 "MAX" above 99999.
#[must_use]
pub fn maintenance_monitor_text(available: i32) -> String {
    if available > MONITOR_MAX {
        "MAX".to_string()
    } else {
        available.to_string()
    }
}

/// Paint one monitor: `DrawTextA` format `0x26`, right aligned and
/// vertically centred on one line, in the faction text color.
fn draw_monitor(
    ctx: &egui::Context,
    layout: CockpitLayout,
    faction: CockpitFaction,
    monitor: ResourceMonitor,
    text: String,
) {
    let rect = logical_rect_to_screen(layout, monitor_rect(faction, monitor));
    let anchor = egui::pos2(
        rect.right() - MONITOR_RIGHT_INSET * layout.scale,
        rect.center().y,
    );
    ctx.layer_painter(egui::LayerId::background()).text(
        anchor,
        egui::Align2::RIGHT_CENTER,
        text,
        egui::FontId::proportional(MONITOR_FONT_HEIGHT * layout.scale),
        faction_text_color(faction),
    );
}

/// Paint the Maintenance monitor.
pub fn draw_maintenance_monitor(
    ctx: &egui::Context,
    layout: CockpitLayout,
    faction: CockpitFaction,
    available: i32,
) {
    draw_monitor(
        ctx,
        layout,
        faction,
        ResourceMonitor::Maintenance,
        maintenance_monitor_text(available),
    );
}

/// Paint the Raw and Refined Materials monitors with the cockpit side's
/// stockpiles (`FUN_00422620`: side `+0x78` and `+0x7c`, commands `0x14c`
/// and `0x14d`).
pub fn draw_material_monitors(
    ctx: &egui::Context,
    layout: CockpitLayout,
    faction: CockpitFaction,
    production: &ProductionState,
) {
    let side = match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    };
    let Some(stock) = production.stock(side) else {
        return;
    };
    for (monitor, value) in [
        (ResourceMonitor::RawMaterials, stock.raw),
        (ResourceMonitor::RefinedMaterials, stock.refined),
    ] {
        draw_monitor(ctx, layout, faction, monitor, value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(
        faction: CockpitFaction,
        available: i32,
    ) -> (CockpitLayout, Vec<egui::epaint::TextShape>) {
        let layout = crate::cockpit::CockpitState::new(faction).layout_for(1280.0, 960.0);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            ..Default::default()
        };
        let shapes = ctx
            .run(input, |ctx| {
                draw_maintenance_monitor(ctx, layout, faction, available);
            })
            .shapes;
        let texts = shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        (layout, texts)
    }

    #[test]
    fn the_cockpits_side_reads_its_own_pools_and_loses_those_in_an_uprising() {
        // FUN_00422620 reads the player's side object; FUN_0050bbb0 blocks
        // the facilities of a system in uprising (+0x88 bit 2).
        use rebellion_core::dat::ExplorationStatus;
        use rebellion_core::ids::{DatId, SectorKey};
        use rebellion_core::uprising::ActiveUprising;
        use rebellion_core::world::{ControlKind, ProductionFacilityInstance, System};

        let mut world = GameWorld::default();
        let pair = |world: &mut GameWorld, side| {
            let system = world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(0x9000_0001),
                name: "Here".into(),
                sector: SectorKey::default(),
                x: 0,
                y: 0,
                exploration_status: ExplorationStatus::Explored,
                popularity_alliance: 0.5,
                popularity_empire: 0.5,
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                espionage_rating: 0.0,
                fleets: Vec::new(),
                ground_units: Vec::new(),
                special_forces: Vec::new(),
                defense_facilities: Vec::new(),
                manufacturing_facilities: Vec::new(),
                production_facilities: Vec::new(),
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control: ControlKind::Controlled(side),
            });
            for (id, is_mine) in [(0x2c00_0001, true), (0x2d00_0002, false)] {
                let key = world
                    .production_facilities
                    .insert(ProductionFacilityInstance {
                        class_dat_id: DatId::new(id),
                        side,
                        is_mine,
                    });
                world.systems[system].production_facilities.push(key);
            }
            system
        };
        pair(&mut world, Faction::Alliance);
        let restless = pair(&mut world, Faction::Alliance);
        pair(&mut world, Faction::Empire);

        let manufacturing = ManufacturingState::new();
        let blockade = BlockadeState::new();
        let mut uprising = UprisingState::new();
        let read = |uprising: &UprisingState, faction| {
            maintenance_available(&world, &manufacturing, &blockade, uprising, faction)
        };
        assert_eq!(read(&uprising, CockpitFaction::Alliance), 100);
        assert_eq!(read(&uprising, CockpitFaction::Empire), 50);

        uprising.active_uprisings.insert(
            restless,
            ActiveUprising {
                started_tick: 1,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        assert_eq!(read(&uprising, CockpitFaction::Alliance), 50);
        assert_eq!(read(&uprising, CockpitFaction::Empire), 50);
    }

    #[test]
    fn monitors_use_the_recovered_rectangles() {
        // FUN_00422ce0 lines 185-258.
        let corner = |faction, monitor| {
            let rect = monitor_rect(faction, monitor);
            (rect.x, rect.y, rect.width, rect.height)
        };
        let alliance = CockpitFaction::Alliance;
        let empire = CockpitFaction::Empire;
        assert_eq!(
            corner(alliance, ResourceMonitor::RawMaterials),
            (265.0, 16.0, 63.0, 17.0)
        );
        assert_eq!(
            corner(alliance, ResourceMonitor::RefinedMaterials),
            (360.0, 16.0, 63.0, 17.0)
        );
        assert_eq!(
            corner(alliance, ResourceMonitor::Maintenance),
            (457.0, 16.0, 63.0, 17.0)
        );
        assert_eq!(
            corner(empire, ResourceMonitor::RawMaterials),
            (175.0, 19.0, 63.0, 17.0)
        );
        assert_eq!(
            corner(empire, ResourceMonitor::RefinedMaterials),
            (273.0, 19.0, 63.0, 17.0)
        );
        assert_eq!(
            corner(empire, ResourceMonitor::Maintenance),
            (371.0, 19.0, 63.0, 17.0)
        );
    }

    #[test]
    fn the_maintenance_monitor_reads_max_above_99999() {
        // FUN_00429200: command 0x14e swaps in TEXTSTRA 4866 when
        // 99999 < value.
        assert_eq!(maintenance_monitor_text(296), "296");
        assert_eq!(maintenance_monitor_text(0), "0");
        assert_eq!(maintenance_monitor_text(-45), "-45");
        assert_eq!(maintenance_monitor_text(99_999), "99999");
        assert_eq!(maintenance_monitor_text(100_000), "MAX");
    }

    #[test]
    fn the_maintenance_monitor_is_right_aligned_and_centred_in_its_rectangle() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let (layout, texts) = texts(faction, 296);
            assert_eq!(texts.len(), 1, "{faction:?}");
            let text = &texts[0];
            assert_eq!(text.galley.text(), "296");
            assert_eq!(text.fallback_color, faction_text_color(faction));

            let rect =
                logical_rect_to_screen(layout, monitor_rect(faction, ResourceMonitor::Maintenance));
            let drawn = text.visual_bounding_rect();
            assert!(
                rect.contains_rect(drawn),
                "{faction:?} {drawn:?} in {rect:?}"
            );
            assert!(
                (rect.right() - drawn.right()).abs() <= 3.0 * layout.scale,
                "{faction:?} right edge {} against {}",
                drawn.right(),
                rect.right()
            );
            assert!(
                (rect.center().y - drawn.center().y).abs() <= 2.0 * layout.scale,
                "{faction:?}"
            );
            let size = MONITOR_FONT_HEIGHT * layout.scale;
            let row = text.galley.size().y;
            assert!(
                (0.9 * size..=1.5 * size).contains(&row),
                "{faction:?} row {row} for font {size}"
            );
        }
    }

    #[test]
    fn the_material_monitors_read_0_on_the_opening_day() {
        // FUN_00422620 writes side +0x78 with command 0x14c and +0x7c with
        // 0x14d; squakenet frames 0119-0149 show both reading 0 on the
        // opening day.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let layout = crate::cockpit::CockpitState::new(faction).layout_for(1280.0, 960.0);
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1040.0),
                )),
                ..Default::default()
            };
            let production = ProductionState::new();
            let shapes = ctx
                .run(input, |ctx| {
                    draw_material_monitors(ctx, layout, faction, &production);
                })
                .shapes;
            let mut drawn: Vec<_> = shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Text(text) => Some(text),
                    _ => None,
                })
                .collect();
            drawn.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
            assert_eq!(drawn.len(), 2, "{faction:?}");
            for (text, monitor) in drawn.iter().zip([
                ResourceMonitor::RawMaterials,
                ResourceMonitor::RefinedMaterials,
            ]) {
                assert_eq!(text.galley.text(), "0");
                let rect = logical_rect_to_screen(layout, monitor_rect(faction, monitor));
                assert!(
                    rect.contains_rect(text.visual_bounding_rect()),
                    "{faction:?} {monitor:?}"
                );
            }
        }
    }
}
