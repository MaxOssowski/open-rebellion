//! Death Star panel: the player's Death Stars and where they are.
//!
//! port: the original has no such panel. A Death Star is an ordinary
//! capital ship (CAPSHPSD record 136) in a fleet, and it destroys a planet
//! through the Fleet menu's Planetary Bombardment, Destroy System (`0x223`,
//! `FUN_0058d990`). This side panel only lists the side's fleets that hold
//! one; it gives no orders.

use egui_macroquad::egui::{self, RichText};
use rebellion_core::ids::FleetKey;
use rebellion_core::missions::MissionFaction;
use rebellion_core::world::GameWorld;

use crate::theme;

/// The side's fleets that hold a Death Star (`Fleet::has_death_star`), in
/// fleet order.
#[must_use]
pub fn death_star_fleets(world: &GameWorld, side: MissionFaction) -> Vec<FleetKey> {
    let is_alliance = side == MissionFaction::Alliance;
    world
        .fleets
        .iter()
        .filter(|(_, fleet)| fleet.is_alliance == is_alliance && fleet.has_death_star)
        .map(|(key, _)| key)
        .collect()
}

/// Draw the Death Star panel as a left-side egui panel. It gives no orders.
pub fn draw_death_star(ctx: &egui::Context, world: &GameWorld, player_faction: MissionFaction) {
    egui::SidePanel::left("death_star_panel")
        .min_width(280.0)
        .max_width(340.0)
        .show(ctx, |ui| {
            ui.heading(RichText::new("Death Star").color(theme::DANGER_RED));
            ui.separator();
            let fleets = death_star_fleets(world, player_faction);
            if fleets.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("No Death Star in your fleets.").color(theme::TEXT_DISABLED),
                );
                return;
            }
            for key in fleets {
                let fleet = &world.fleets[key];
                let name = world.fleet_name(key).unwrap_or("Fleet");
                let location = world
                    .systems
                    .get(fleet.location)
                    .map_or("Unknown", |s| s.name.as_str());
                ui.group(|ui| {
                    ui.label(
                        RichText::new(name)
                            .color(theme::TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("Location: {location}"))
                            .color(theme::TEXT_SECONDARY)
                            .size(10.0),
                    );
                    ui.label(
                        RichText::new(format!("Capital ships: {}", fleet.ship_count()))
                            .color(theme::TEXT_SECONDARY)
                            .size(10.0),
                    );
                });
                ui.add_space(4.0);
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Destroy System is ordered from the fleet's Planetary Bombardment menu.",
                )
                .color(theme::TEXT_SECONDARY)
                .size(10.0),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::world::Fleet;

    fn fleet(world: &mut GameWorld, is_alliance: bool, has_death_star: bool) -> FleetKey {
        world.fleets.insert(Fleet {
            location: Default::default(),
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star,
        })
    }

    #[test]
    fn the_panel_lists_only_the_side_s_fleets_holding_a_death_star() {
        let mut world = GameWorld::default();
        let imperial = fleet(&mut world, false, true);
        fleet(&mut world, false, false);
        let rebel = fleet(&mut world, true, true);
        assert_eq!(death_star_fleets(&world, MissionFaction::Empire), [imperial]);
        assert_eq!(death_star_fleets(&world, MissionFaction::Alliance), [rebel]);
    }
}
