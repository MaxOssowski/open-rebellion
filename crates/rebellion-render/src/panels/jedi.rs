//! Force panel: the player's characters with a Force, from the original's
//! fields (`ghidra/notes/force-awareness.md`, `jedi-rs-replacement.md`).
//!
//! port: the original has no such panel; it shows a character's Force only
//! in the Status window's "Force Ranking:" row (`FUN_004486f0`). This side
//! panel lists the same value for each character, read-only: training is
//! the Jedi Training mission.

use egui_macroquad::egui::{self, Color32, RichText, ScrollArea};
use rebellion_core::ids::CharacterKey;
use rebellion_core::missions::MissionFaction;
use rebellion_core::world::{Character, GameWorld};

use super::PanelAction;
use crate::status_window::force_ranking;
use crate::theme;

/// Mutable UI state for the Force panel.
#[derive(Debug, Clone, Default)]
pub struct JediPanelState {
    /// If set, show detail view for this character.
    pub focused: Option<CharacterKey>,
}

/// A character's Force as the original keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceSummary {
    /// Force `+0x8c`: `ForceExperience` `+0x8e` plus `ForceTraining`
    /// `+0x90` ([`Character::force_level`]).
    pub level: u32,
    /// The Status window's word for it (`FUN_004486f0`).
    pub ranking: &'static str,
    /// `ForceAware` (`+0xac` bit `0x10`).
    pub aware: bool,
    /// `ForcePotential` (`+0xac` bit `0x20`).
    pub potential: bool,
    /// The trainer who discovered it (`+0xa4`, `force_discovery.rs`).
    pub discoverer: Option<String>,
}

/// Whether `character` has a Force value, `ForceAware` or `ForcePotential`.
#[must_use]
pub fn has_force(character: &Character) -> bool {
    character.force_level() > 0 || character.force_aware || character.force_potential
}

/// The ranking a roster badge shows: the Status window's word once it is
/// above "None" (Force 10 or more), else nothing.
#[must_use]
pub fn ranking_badge(level: u32) -> Option<&'static str> {
    (level >= 10).then(|| force_ranking(level))
}

/// `character`'s Force, or `None` for a character without one
/// ([`has_force`]).
#[must_use]
pub fn force_summary(world: &GameWorld, character: &Character) -> Option<ForceSummary> {
    if !has_force(character) {
        return None;
    }
    let level = character.force_level();
    let discoverer = character.force_discoverer.and_then(|id| {
        world
            .characters
            .values()
            .find(|c| c.dat_id == id)
            .map(|c| c.name.clone())
    });
    Some(ForceSummary {
        level,
        ranking: force_ranking(level),
        aware: character.force_aware,
        potential: character.force_potential,
        discoverer,
    })
}

/// The panel's rows: the side's living characters with a Force, the
/// strongest first, then by name.
#[must_use]
pub fn force_rows(world: &GameWorld, is_alliance: bool) -> Vec<(CharacterKey, ForceSummary)> {
    let mut rows: Vec<(CharacterKey, ForceSummary)> = world
        .characters
        .iter()
        .filter(|(_, c)| c.is_alliance == is_alliance && !c.is_killed)
        .filter_map(|(key, c)| force_summary(world, c).map(|summary| (key, summary)))
        .collect();
    rows.sort_by(|(a, x), (b, y)| {
        y.level
            .cmp(&x.level)
            .then_with(|| world.characters[*a].name.cmp(&world.characters[*b].name))
    });
    rows
}

/// The colour of a ranking's label.
fn ranking_color(level: u32) -> Color32 {
    match level {
        0..=9 => Color32::from_gray(120),
        10..=79 => Color32::from_rgb(100, 160, 255),
        80..=119 => Color32::from_rgb(180, 120, 255),
        _ => Color32::from_rgb(60, 200, 100),
    }
}

/// Draw the Force panel as a left-side egui panel.
pub fn draw_jedi(
    ctx: &egui::Context,
    world: &GameWorld,
    _state: &mut JediPanelState,
    player_faction: MissionFaction,
) -> Option<PanelAction> {
    let is_alliance = player_faction == MissionFaction::Alliance;
    egui::SidePanel::left("jedi_panel")
        .min_width(280.0)
        .max_width(340.0)
        .show(ctx, |ui| {
            ui.heading(RichText::new("The Force").color(theme::GOLD));
            ui.separator();
            let rows = force_rows(world, is_alliance);
            if rows.is_empty() {
                ui.add_space(20.0);
                ui.label(RichText::new("No character with a Force.").color(theme::TEXT_DISABLED));
                return;
            }
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (key, summary) in &rows {
                        let character = &world.characters[*key];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&character.name)
                                        .color(theme::TEXT_PRIMARY)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(summary.ranking)
                                        .color(ranking_color(summary.level))
                                        .size(10.0),
                                );
                            });
                            ui.label(
                                RichText::new(format!("Force: {}", summary.level))
                                    .color(theme::TEXT_SECONDARY)
                                    .size(10.0),
                            );
                            let mut marks = Vec::new();
                            if summary.aware {
                                marks.push("Force aware");
                            }
                            if summary.potential {
                                marks.push("Force potential");
                            }
                            if !marks.is_empty() {
                                ui.label(
                                    RichText::new(marks.join(", "))
                                        .color(theme::TEXT_SECONDARY)
                                        .size(10.0),
                                );
                            }
                            if let Some(name) = &summary.discoverer {
                                ui.label(
                                    RichText::new(format!("Discovered by {name}"))
                                        .color(theme::TEXT_SECONDARY)
                                        .size(10.0),
                                );
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
        });
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::ids::DatId;

    fn person(name: &str, is_alliance: bool, edit: impl FnOnce(&mut Character)) -> Character {
        let mut c = Character {
            name: name.into(),
            is_alliance,
            is_empire: !is_alliance,
            ..Character::default()
        };
        edit(&mut c);
        c
    }

    #[test]
    fn the_force_panel_lists_the_side_s_force_values_strongest_first() {
        // Force +0x8c = +0x8e + +0x90 (Character::force_level); the word is
        // the Status window's (FUN_004486f0: <10 None, <20 Novice, <80
        // Trainee, <100 Jedi Student, <120 Jedi Knight, else Jedi Master).
        let mut world = GameWorld::default();
        let yoda = world.characters.insert(person("Yoda", true, |c| {
            c.dat_id = DatId::new(0x3000_0243);
            c.force_base = 150;
        }));
        let luke = world.characters.insert(person("Luke", true, |c| {
            c.force_base = 20;
            c.force_training = 5;
            c.force_aware = true;
            c.force_potential = true;
            c.force_discoverer = Some(DatId::new(0x3000_0243));
        }));
        let latent = world.characters.insert(person("Latent", true, |c| {
            c.force_potential = true;
        }));
        world.characters.insert(person("Han", true, |_| {}));
        world
            .characters
            .insert(person("Vader", false, |c| c.force_base = 140));
        world.characters.insert(person("Gone", true, |c| {
            c.force_base = 90;
            c.is_killed = true;
        }));

        let rows = force_rows(&world, true);
        assert_eq!(
            rows.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            [yoda, luke, latent]
        );
        assert_eq!(
            rows[1].1,
            ForceSummary {
                level: 25,
                ranking: "Trainee",
                aware: true,
                potential: true,
                discoverer: Some("Yoda".into()),
            }
        );
        assert_eq!(rows[0].1.ranking, "Jedi Master");
        assert_eq!((rows[2].1.level, rows[2].1.ranking), (0, "None"));
        assert_eq!(force_rows(&world, false).len(), 1);
    }

    #[test]
    fn a_roster_badge_needs_a_force_and_names_a_ranking_from_ten() {
        let none = person("Han", true, |_| {});
        assert!(!has_force(&none));
        for edit in [
            (|c: &mut Character| c.force_base = 1) as fn(&mut Character),
            |c| c.force_training = 1,
            |c| c.force_aware = true,
            |c| c.force_potential = true,
        ] {
            assert!(has_force(&person("Luke", true, edit)));
        }
        assert_eq!(ranking_badge(9), None);
        assert_eq!(ranking_badge(10), Some("Novice"));
        assert_eq!(ranking_badge(120), Some("Jedi Master"));
    }
}
