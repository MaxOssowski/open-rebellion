//! Encyclopedia viewer — browse ships, characters, and systems with artwork.
//!
//! Displays a floating egui window with tabs for capital ships, fighters,
//! characters, and star systems.  Each entity shows its 400×200 BMP artwork
//! loaded from `EData`/ alongside stats pulled from `GameWorld`.
//!
//! # EDATA mapping
//!
//! The original game stores encyclopedia images in sequentially numbered BMP
//! files (`EData/EDATA.NNN`).  The C# editor (`SwRebellionEditor`) reveals the
//! direct index mapping for entity types that don't go through `ENCYBMAP.DLL`:
//!
//! | Entity type          | First EDATA index |
//! |----------------------|-------------------|
//! | Production facilities | 1               |
//! | Manufacturing facs   | 3                 |
//! | Troops               | 15                |
//! | Special forces       | 25                |
//! | Fighters             | 34                |
//! | Capital ships        | 42                |
//! | Major characters     | 72                |
//! | Minor characters     | 78                |
//!
//! Star systems use a two-level lookup via `ENCYBMAP.DLL` (not yet implemented
//! here — systems fall back to a placeholder image).
//!
//! # Integration
//!
//! ```ignore
//! egui_macroquad::ui(|ctx| {
//!     if let Some(action) = draw_encyclopedia(ctx, world, &mut enc_state) {
//!         // handle action (currently none, may add focus-system in future)
//!     }
//! });
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui_macroquad::egui::TextureOptions;
use egui_macroquad::egui::{self, Color32, RichText, ScrollArea, TextureHandle, Vec2};
use rebellion_core::ids::{CapitalShipKey, CharacterKey, FighterKey, SystemKey};
use rebellion_core::world::GameWorld;

#[cfg(not(target_arch = "wasm32"))]
use crate::bmp_cache::{load_approved_hd_assets, validated_hd_bytes};
use crate::bmp_cache::{ApprovedHdAsset, AssetRenderProfile, BmpCache, DllSource};
use crate::cockpit::CockpitFaction;

#[cfg(target_arch = "wasm32")]
static WASM_EDATA_CACHE: std::sync::LazyLock<std::sync::Mutex<HashMap<String, Vec<u8>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Install original encyclopedia artwork unpacked from the browser runtime pack.
///
/// Keys are original filenames such as `EDATA.042`. GPU textures remain lazy:
/// opening a topic decodes only that topic's source bitmap, then retains the
/// resulting egui texture in [`EncyclopediaState`].
#[cfg(target_arch = "wasm32")]
pub fn set_encyclopedia_asset_cache(cache: HashMap<String, Vec<u8>>) {
    *WASM_EDATA_CACHE.lock().unwrap() = cache;
}

fn edata_filename(edata_n: u16) -> String {
    format!("EDATA.{edata_n:03}")
}

#[cfg(target_arch = "wasm32")]
fn wasm_edata_bytes(edata_n: u16) -> Option<Vec<u8>> {
    WASM_EDATA_CACHE
        .lock()
        .unwrap()
        .get(&edata_filename(edata_n))
        .cloned()
}

// ---------------------------------------------------------------------------
// Original index shell
// ---------------------------------------------------------------------------

/// Native resource size used by `FUN_004665f0` and the topic-child constructor.
/// `FUN_00429f30` requests a 470 by 330 outer window, so final production
/// clipping remains part of the topic/catalog integration gate.
pub const ENCYCLOPEDIA_WIDTH: f32 = 470.0;
pub const ENCYCLOPEDIA_HEIGHT: f32 = 331.0;

const INDEX_CONTENT_X: f32 = 12.0;
const INDEX_CONTENT_Y: f32 = 13.0;
const INDEX_CONTENT_WIDTH: f32 = 400.0;
const INDEX_CONTENT_HEIGHT: f32 = 306.0;
const INDEX_CONTROLS_X: f32 = 22.0;
const INDEX_CONTROLS_Y: f32 = 46.0;

const INDEX_BASE_ALLIANCE: u32 = 10_335;
const INDEX_BASE_EMPIRE: u32 = 10_336;
const INDEX_RAIL_ALLIANCE: u32 = 10_820;
const INDEX_RAIL_EMPIRE: u32 = 10_821;
const INDEX_CONTENT: u32 = 10_822;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EncyclopediaControlSpec {
    command_id: u16,
    x: u16,
    width: u16,
    normal_resource: u32,
    pressed_resource: u32,
}

const fn encyclopedia_control_specs(faction: CockpitFaction) -> [EncyclopediaControlSpec; 10] {
    let (
        faction_normal,
        faction_pressed,
        ships_normal,
        ships_pressed,
        forces_normal,
        forces_pressed,
        characters_normal,
        characters_pressed,
    ) = match faction {
        CockpitFaction::Alliance => (
            10_832, 10_833, 10_840, 10_841, 10_842, 10_843, 10_846, 10_847,
        ),
        CockpitFaction::Empire => (
            10_834, 10_835, 10_838, 10_839, 10_844, 10_845, 10_848, 10_849,
        ),
    };
    [
        EncyclopediaControlSpec {
            command_id: 0x79,
            x: 0,
            width: 36,
            normal_resource: 10_830,
            pressed_resource: 10_831,
        },
        EncyclopediaControlSpec {
            command_id: 0x7a,
            x: 38,
            width: 36,
            normal_resource: faction_normal,
            pressed_resource: faction_pressed,
        },
        EncyclopediaControlSpec {
            command_id: 0x7b,
            x: 76,
            width: 36,
            normal_resource: ships_normal,
            pressed_resource: ships_pressed,
        },
        EncyclopediaControlSpec {
            command_id: 0x7c,
            x: 114,
            width: 35,
            normal_resource: forces_normal,
            pressed_resource: forces_pressed,
        },
        EncyclopediaControlSpec {
            command_id: 0x7d,
            x: 151,
            width: 36,
            normal_resource: 10_836,
            pressed_resource: 10_837,
        },
        EncyclopediaControlSpec {
            command_id: 0x7e,
            x: 189,
            width: 36,
            normal_resource: 10_852,
            pressed_resource: 10_853,
        },
        EncyclopediaControlSpec {
            command_id: 0x7f,
            x: 227,
            width: 34,
            normal_resource: 10_856,
            pressed_resource: 10_857,
        },
        EncyclopediaControlSpec {
            command_id: 0x80,
            x: 263,
            width: 36,
            normal_resource: 10_854,
            pressed_resource: 10_855,
        },
        EncyclopediaControlSpec {
            command_id: 0x81,
            x: 301,
            width: 35,
            normal_resource: 10_850,
            pressed_resource: 10_851,
        },
        EncyclopediaControlSpec {
            command_id: 0x82,
            x: 338,
            width: 37,
            normal_resource: characters_normal,
            pressed_resource: characters_pressed,
        },
    ]
}

/// Paint the recovered Encyclopedia index shell and its ten original category
/// controls.
///
/// `FUN_004665f0` composes the faction base (`10335`/`10336`), faction rail
/// (`10820`/`10821`), index content (`10822`), and command `0x79..0x82`
/// buttons at the coordinates retained here. Topic labels, catalog ordering,
/// and list contents remain a separate source-recovery gate.
pub fn draw_encyclopedia_index_shell(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
) -> Option<u16> {
    if scale <= 0.0 {
        return None;
    }
    let mut selected = None;
    egui::Area::new(egui::Id::new("original-encyclopedia-index-shell"))
        .fixed_pos(origin)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let size = egui::vec2(ENCYCLOPEDIA_WIDTH * scale, ENCYCLOPEDIA_HEIGHT * scale);
            let (window_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let (base, rail) = match faction {
                CockpitFaction::Alliance => (INDEX_BASE_ALLIANCE, INDEX_RAIL_ALLIANCE),
                CockpitFaction::Empire => (INDEX_BASE_EMPIRE, INDEX_RAIL_EMPIRE),
            };
            paint_strategy_resource(ui.painter(), ctx, cache, base, window_rect);
            paint_strategy_resource(
                ui.painter(),
                ctx,
                cache,
                rail,
                encyclopedia_rect(window_rect, scale, 412.0, 0.0, 58.0, 330.0),
            );
            paint_strategy_resource(
                ui.painter(),
                ctx,
                cache,
                INDEX_CONTENT,
                encyclopedia_rect(
                    window_rect,
                    scale,
                    INDEX_CONTENT_X,
                    INDEX_CONTENT_Y,
                    INDEX_CONTENT_WIDTH,
                    INDEX_CONTENT_HEIGHT,
                ),
            );

            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            for control in encyclopedia_control_specs(faction) {
                let rect = encyclopedia_rect(
                    window_rect,
                    scale,
                    INDEX_CONTROLS_X + f32::from(control.x),
                    INDEX_CONTROLS_Y,
                    f32::from(control.width),
                    41.0,
                );
                let response = ui.interact(
                    rect,
                    ui.id().with(("encyclopedia-category", control.command_id)),
                    egui::Sense::click(),
                );
                let pressed = primary_down
                    && pointer.is_some_and(|point| encyclopedia_rect_contains(rect, point));
                paint_strategy_resource(
                    ui.painter(),
                    ctx,
                    cache,
                    if pressed {
                        control.pressed_resource
                    } else {
                        control.normal_resource
                    },
                    rect,
                );
                if response.clicked()
                    && response
                        .interact_pointer_pos()
                        .is_some_and(|point| encyclopedia_rect_contains(rect, point))
                {
                    selected = Some(control.command_id);
                }
            }
        });
    selected
}

fn encyclopedia_rect(
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(parent.min.x + x * scale, parent.min.y + y * scale),
        egui::vec2(width * scale, height * scale),
    )
}

fn encyclopedia_rect_contains(rect: egui::Rect, point: egui::Pos2) -> bool {
    point.x >= rect.min.x && point.x < rect.max.x && point.y >= rect.min.y && point.y < rect.max.y
}

fn paint_strategy_resource(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    resource_id: u32,
    rect: egui::Rect,
) {
    let Some(texture) = cache.get(ctx, DllSource::Strategy, resource_id) else {
        return;
    };
    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Fixed-position test adapter for deterministic browser inspection.
#[cfg(feature = "interface-test-fixtures")]
pub fn draw_encyclopedia_index_fixture(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
) {
    let _ = draw_encyclopedia_index_shell(ctx, cache, faction, egui::pos2(85.0, 55.0), 1.0);
}

// ---------------------------------------------------------------------------
// Tab selection
// ---------------------------------------------------------------------------

/// Which entity category is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncyclopediaTab {
    #[default]
    CapitalShips,
    Fighters,
    Characters,
    Systems,
}

// ---------------------------------------------------------------------------
// EncyclopediaState
// ---------------------------------------------------------------------------

/// All mutable state for the encyclopedia panel.
pub struct EncyclopediaState {
    /// Whether the panel is open.
    pub open: bool,
    /// Active tab.
    pub tab: EncyclopediaTab,
    /// Selected entity within the current tab (list index).
    pub selected_index: usize,
    /// Path to the `EData`/ directory (original BMPs).
    pub edata_path: Option<PathBuf>,
    /// Path to HD upscaled PNGs directory, used only by the faithful-HD profile.
    pub hd_path: Option<PathBuf>,
    /// Explicit asset profile. Original parity is the default.
    pub asset_profile: AssetRenderProfile,
    /// EDATA keys explicitly approved by the faithful-HD manifest.
    approved_hd_assets: HashMap<String, ApprovedHdAsset>,
    /// Cached textures keyed by EDATA file number (1-based).
    textures: HashMap<u16, Option<TextureHandle>>,
}

impl EncyclopediaState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the `EData` directory.  Call before opening the encyclopedia.
    pub fn set_edata_path(&mut self, path: impl Into<PathBuf>) {
        self.edata_path = Some(path.into());
        self.textures.clear();
    }

    /// Configure the HD upscaled PNG directory. Expected naming:
    /// `EDATA_NNN.png`. Setting a path does not enable HD substitution.
    pub fn set_hd_path(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let manifest_root = path.parent().unwrap_or(&path);
            self.approved_hd_assets = load_approved_hd_assets(manifest_root);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.approved_hd_assets.clear();
        }
        self.hd_path = Some(path);
        self.textures.clear();
    }

    /// Change the explicit render profile and invalidate prior textures.
    pub fn set_asset_profile(&mut self, profile: AssetRenderProfile) {
        if self.asset_profile != profile {
            self.asset_profile = profile;
            self.textures.clear();
        }
    }
}

impl Default for EncyclopediaState {
    fn default() -> Self {
        EncyclopediaState {
            open: false,
            tab: EncyclopediaTab::default(),
            selected_index: 0,
            edata_path: None,
            hd_path: None,
            asset_profile: AssetRenderProfile::OriginalParity,
            approved_hd_assets: HashMap::new(),
            textures: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public draw function
// ---------------------------------------------------------------------------

/// Render the encyclopedia window inside an `egui_macroquad::ui` closure.
///
/// Returns `Some(SystemKey)` when the user clicks "Zoom to system" on a
/// star system entry (caller should pan the galaxy map to that system).
/// Returns `None` otherwise.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
pub fn draw_encyclopedia(
    ctx: &egui::Context,
    world: &GameWorld,
    state: &mut EncyclopediaState,
    bmp_cache: &mut BmpCache,
) -> Option<SystemKey> {
    if !state.open {
        return None;
    }

    let mut focus_system: Option<SystemKey> = None;

    // Extract fields that the Window::open() needs to borrow independently
    // of the closure's mutable borrow of `state`.
    let mut window_open = state.open;

    egui::Window::new("Encyclopedia")
        .default_size([700.0, 520.0])
        .min_width(480.0)
        .min_height(360.0)
        .collapsible(true)
        .open(&mut window_open)
        .show(ctx, |ui| {
            // ── Tab bar ───────────────────────────────────────────────────
            ui.horizontal(|ui| {
                for (label, tab) in [
                    ("Capital Ships", EncyclopediaTab::CapitalShips),
                    ("Fighters", EncyclopediaTab::Fighters),
                    ("Characters", EncyclopediaTab::Characters),
                    ("Systems", EncyclopediaTab::Systems),
                ] {
                    if ui.selectable_label(state.tab == tab, label).clicked() && state.tab != tab {
                        state.tab = tab;
                        state.selected_index = 0;
                    }
                }
            });
            ui.separator();

            // ── Two-column layout: list (left) | detail (right) ───────────
            ui.columns(2, |cols| {
                // Left: scrollable entity list
                let list_ui = &mut cols[0];
                ScrollArea::vertical()
                    .id_salt("enc_list")
                    .show(list_ui, |ui| match state.tab {
                        EncyclopediaTab::CapitalShips => {
                            let keys: Vec<(CapitalShipKey, &str)> = world
                                .capital_ship_classes
                                .iter()
                                .map(|(k, c)| (k, c.name.as_str()))
                                .collect();
                            for (i, (_, name)) in keys.iter().enumerate() {
                                let sel = state.selected_index == i;
                                if ui.selectable_label(sel, *name).clicked() {
                                    state.selected_index = i;
                                }
                            }
                        }
                        EncyclopediaTab::Fighters => {
                            let keys: Vec<(FighterKey, &str)> = world
                                .fighter_classes
                                .iter()
                                .map(|(k, c)| (k, c.name.as_str()))
                                .collect();
                            for (i, (_, name)) in keys.iter().enumerate() {
                                let sel = state.selected_index == i;
                                if ui.selectable_label(sel, *name).clicked() {
                                    state.selected_index = i;
                                }
                            }
                        }
                        EncyclopediaTab::Characters => {
                            let chars: Vec<(CharacterKey, &str)> = world
                                .characters
                                .iter()
                                .map(|(k, c)| (k, c.name.as_str()))
                                .collect();
                            for (i, (_, name)) in chars.iter().enumerate() {
                                let sel = state.selected_index == i;
                                if ui.selectable_label(sel, *name).clicked() {
                                    state.selected_index = i;
                                }
                            }
                        }
                        EncyclopediaTab::Systems => {
                            let systems: Vec<(SystemKey, &str)> = world
                                .systems
                                .iter()
                                .map(|(k, s)| (k, s.name.as_str()))
                                .collect();
                            for (i, (_, name)) in systems.iter().enumerate() {
                                let sel = state.selected_index == i;
                                if ui.selectable_label(sel, *name).clicked() {
                                    state.selected_index = i;
                                }
                            }
                        }
                    });

                // Right: detail pane
                let detail_ui = &mut cols[1];
                ScrollArea::vertical()
                    .id_salt("enc_detail")
                    .show(detail_ui, |ui| {
                        match state.tab {
                            EncyclopediaTab::CapitalShips => {
                                let ships: Vec<CapitalShipKey> =
                                    world.capital_ship_classes.keys().collect();
                                if let Some(&key) = ships.get(state.selected_index) {
                                    if let Some(ship) = world.capital_ship_classes.get(key) {
                                        // GOKRES.DLL 122×50 ship status sprite.
                                        // Formula: resource_id = dat_id.raw() + 1024.
                                        // Ships without a sprite fall through to the EDATA image.
                                        let gokres_id = ship.dat_id.raw() + 1024;
                                        if let Some(tex) =
                                            bmp_cache.get(ctx, DllSource::Gokres, gokres_id)
                                        {
                                            ui.add(
                                                egui::Image::new(tex)
                                                    .fit_to_exact_size(Vec2::new(122.0, 50.0)),
                                            );
                                        }

                                        // EDATA offset for capital ships: 42 + 0-based index
                                        let edata_n = 42u16 + state.selected_index as u16;
                                        show_edata_image(ui, ctx, edata_n, state);
                                        ui.add_space(4.0);
                                        ui.heading(&ship.name);
                                        ui.separator();
                                        let faction = match (ship.is_alliance, ship.is_empire) {
                                            (true, false) => "Alliance",
                                            (false, true) => "Empire",
                                            _ => "Both",
                                        };
                                        stat_row(ui, "Faction", faction);
                                        stat_row(ui, "Hull", &ship.hull.to_string());
                                        stat_row(ui, "Shields", &ship.shield_strength.to_string());
                                        stat_row(
                                            ui,
                                            "Sublight",
                                            &ship.sub_light_engine.to_string(),
                                        );
                                        stat_row(ui, "Hyperdrive", &ship.hyperdrive.to_string());
                                        stat_row(ui, "Maneuver", &ship.maneuverability.to_string());
                                        stat_row(
                                            ui,
                                            "Fighters",
                                            &ship.fighter_capacity.to_string(),
                                        );
                                        stat_row(ui, "Troops", &ship.troop_capacity.to_string());
                                        stat_row(
                                            ui,
                                            "Build cost",
                                            &ship.refined_material_cost.to_string(),
                                        );
                                        stat_row(
                                            ui,
                                            "Maintenance",
                                            &ship.maintenance_cost.to_string(),
                                        );
                                        stat_row(
                                            ui,
                                            "Research order",
                                            &ship.research_order.to_string(),
                                        );
                                        stat_row(
                                            ui,
                                            "Build time",
                                            &ship.research_difficulty.to_string(),
                                        );
                                    }
                                }
                            }
                            EncyclopediaTab::Fighters => {
                                let fighters: Vec<FighterKey> =
                                    world.fighter_classes.keys().collect();
                                if let Some(&key) = fighters.get(state.selected_index) {
                                    if let Some(ftr) = world.fighter_classes.get(key) {
                                        // EDATA offset for fighters: 34 + 0-based index
                                        let edata_n = 34u16 + state.selected_index as u16;
                                        show_edata_image(ui, ctx, edata_n, state);
                                        ui.add_space(4.0);
                                        ui.heading(&ftr.name);
                                        ui.separator();
                                        let faction = match (ftr.is_alliance, ftr.is_empire) {
                                            (true, false) => "Alliance",
                                            (false, true) => "Empire",
                                            _ => "Both",
                                        };
                                        stat_row(ui, "Faction", faction);
                                        stat_row(
                                            ui,
                                            "Squadron size",
                                            &ftr.squadron_size.to_string(),
                                        );
                                        stat_row(ui, "Torpedoes", &ftr.torpedoes.to_string());
                                        stat_row(
                                            ui,
                                            "Build cost",
                                            &ftr.refined_material_cost.to_string(),
                                        );
                                        stat_row(
                                            ui,
                                            "Maintenance",
                                            &ftr.maintenance_cost.to_string(),
                                        );
                                    }
                                }
                            }
                            EncyclopediaTab::Characters => {
                                let chars: Vec<CharacterKey> = world.characters.keys().collect();
                                if let Some(&key) = chars.get(state.selected_index) {
                                    if let Some(chr) = world.characters.get(key) {
                                        // Major characters (0..5) → EDATA 72+; minor (6+) → 78+
                                        // We don't have a major/minor flag split by index here,
                                        // but world.characters stores major first (load order).
                                        // Use 72 for first 6, 78 for the rest.
                                        let edata_n = if state.selected_index < 6 {
                                            72u16 + state.selected_index as u16
                                        } else {
                                            78u16 + (state.selected_index - 6) as u16
                                        };
                                        show_edata_image(ui, ctx, edata_n, state);
                                        ui.add_space(4.0);
                                        ui.heading(&chr.name);
                                        ui.separator();
                                        let kind = if chr.is_major {
                                            "Major character"
                                        } else {
                                            "Minor character"
                                        };
                                        stat_row(ui, "Type", kind);
                                        stat_row_pair(
                                            ui,
                                            "Diplomacy",
                                            chr.diplomacy.base,
                                            chr.diplomacy.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Espionage",
                                            chr.espionage.base,
                                            chr.espionage.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Ship Design",
                                            chr.ship_design.base,
                                            chr.ship_design.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Troop Training",
                                            chr.troop_training.base,
                                            chr.troop_training.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Facility Design",
                                            chr.facility_design.base,
                                            chr.facility_design.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Combat",
                                            chr.combat.base,
                                            chr.combat.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Leadership",
                                            chr.leadership.base,
                                            chr.leadership.variance,
                                        );
                                        stat_row_pair(
                                            ui,
                                            "Loyalty",
                                            chr.loyalty.base,
                                            chr.loyalty.variance,
                                        );
                                        if chr.jedi_probability > 0 {
                                            stat_row(
                                                ui,
                                                "Jedi probability",
                                                &format!("{}%", chr.jedi_probability),
                                            );
                                        }
                                        let mut roles = Vec::new();
                                        if chr.can_be_admiral {
                                            roles.push("Admiral");
                                        }
                                        if chr.can_be_general {
                                            roles.push("General");
                                        }
                                        if chr.can_be_commander {
                                            roles.push("Commander");
                                        }
                                        if !roles.is_empty() {
                                            stat_row(ui, "Roles", &roles.join(", "));
                                        }
                                    }
                                }
                            }
                            EncyclopediaTab::Systems => {
                                let systems: Vec<SystemKey> = world.systems.keys().collect();
                                if let Some(&key) = systems.get(state.selected_index) {
                                    if let Some(system) = world.systems.get(key) {
                                        // Systems use ENCYBMAP.DLL for their image index.
                                        // Until ENCYBMAP is parsed, show a placeholder.
                                        show_placeholder_image(ui);
                                        ui.add_space(4.0);
                                        ui.heading(&system.name);
                                        ui.separator();
                                        if let Some(sector) = world.sectors.get(system.sector) {
                                            stat_row(ui, "Sector", &sector.name);
                                            let region = match sector.group {
                                                rebellion_core::dat::SectorGroup::Core => "Core",
                                                rebellion_core::dat::SectorGroup::RimInner => {
                                                    "Inner Rim"
                                                }
                                                rebellion_core::dat::SectorGroup::RimOuter => {
                                                    "Outer Rim"
                                                }
                                            };
                                            stat_row(ui, "Region", region);
                                        }
                                        stat_row(
                                            ui,
                                            "Position",
                                            &format!("({}, {})", system.x, system.y),
                                        );
                                        stat_row(
                                            ui,
                                            "Alliance",
                                            &format!("{:.0}%", system.popularity_alliance * 100.0),
                                        );
                                        stat_row(
                                            ui,
                                            "Empire",
                                            &format!("{:.0}%", system.popularity_empire * 100.0),
                                        );
                                        stat_row(ui, "Fleets", &system.fleets.len().to_string());
                                        stat_row(
                                            ui,
                                            "Defenses",
                                            &system.defense_facilities.len().to_string(),
                                        );
                                        stat_row(
                                            ui,
                                            "Shipyards",
                                            &system.manufacturing_facilities.len().to_string(),
                                        );

                                        ui.add_space(6.0);
                                        if ui.small_button("Zoom to system on map").clicked() {
                                            focus_system = Some(key);
                                        }
                                    }
                                }
                            }
                        }
                    });
            });
        });

    // Write back the open flag (egui sets it to false when the X button is clicked).
    state.open = window_open;

    focus_system
}

// ---------------------------------------------------------------------------
// Image helpers
// ---------------------------------------------------------------------------

/// Load and display an EDATA BMP image, caching the texture by EDATA number.
///
/// On first call for a given `edata_n`, reads the BMP file, decodes it via
/// the `image` crate, and registers it as an egui texture.  Subsequent calls
/// use the cached handle.  If the file is missing or fails to decode, shows
/// a gray placeholder rectangle.
fn show_edata_image(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    edata_n: u16,
    state: &mut EncyclopediaState,
) {
    // Lazy-load the texture if not yet cached. HD is an explicit enhancement
    // profile; original parity never probes the HD tree.
    if !state.textures.contains_key(&edata_n) {
        let handle = load_edata_texture(
            ctx,
            edata_n,
            state.hd_path.as_deref(),
            state.edata_path.as_deref(),
            state.asset_profile,
            state
                .approved_hd_assets
                .get(&format!("edata/EDATA_{edata_n:03}")),
        );
        if handle.is_none() {
            eprintln!(
                "[encyclopedia] original artwork unavailable asset={}",
                edata_filename(edata_n)
            );
        }
        state.textures.insert(edata_n, handle);
    }

    if let Some(Some(handle)) = state.textures.get(&edata_n) {
        let size = egui::vec2(400.0, 200.0);
        ui.add(egui::Image::from_texture((handle.id(), size)));
    } else {
        show_placeholder_image(ui);
    }
}

/// Paint one original EDATA image at a fixed native-size location for the
/// test-only browser transport gate.
///
/// This is not an encyclopedia UI and is absent from production builds. It
/// isolates the runtime-pack/cache/decode/render path so exact source pixels
/// can be compared without promoting the current replacement window.
#[cfg(feature = "interface-test-fixtures")]
pub fn draw_encyclopedia_artwork_fixture(
    ctx: &egui::Context,
    edata_n: u16,
    state: &mut EncyclopediaState,
) {
    egui::Area::new(egui::Id::new("encyclopedia_artwork_transport_fixture"))
        .fixed_pos(egui::pos2(120.0, 120.0))
        .show(ctx, |ui| show_edata_image(ui, ctx, edata_n, state));
}

/// Draw a gray placeholder rectangle when no image is available.
fn show_placeholder_image(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(400.0, 200.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(40));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "No image",
        egui::FontId::default(),
        Color32::from_gray(100),
    );
}

/// Load an EDATA image and register it as an egui texture.
///
/// In faithful-HD mode, checks for an approved PNG first (`EDATA_NNN.png` in
/// `hd_path`), then falls back to the original BMP (`EDATA.NNN` in
/// `edata_path`). Original-parity mode never probes the HD path.
/// Returns `None` if neither exists or decoding fails.
/// On WASM targets, always returns `None` (filesystem access not available).
#[cfg(not(target_arch = "wasm32"))]
fn load_edata_texture(
    ctx: &egui::Context,
    edata_n: u16,
    hd_path: Option<&Path>,
    edata_path: Option<&Path>,
    profile: AssetRenderProfile,
    approved_hd: Option<&ApprovedHdAsset>,
) -> Option<TextureHandle> {
    let dir = edata_path?;
    let bmp_file = dir.join(edata_filename(edata_n));

    if profile == AssetRenderProfile::FaithfulHd {
        if let Some(hd_dir) = hd_path {
            let hd_file = hd_dir.join(format!("EDATA_{edata_n:03}.png"));
            if let Some(bytes) =
                approved_hd.and_then(|approval| validated_hd_bytes(&bmp_file, &hd_file, approval))
            {
                if let Some(handle) = load_image_bytes(ctx, edata_n, &bytes, TextureOptions::LINEAR)
                {
                    return Some(handle);
                }
            } else if approved_hd.is_some() && hd_file.exists() {
                eprintln!(
                    "[encyclopedia] HD source/output digest mismatch for EDATA_{edata_n:03}; falling back to original"
                );
            }
        }
    }

    // Original data is authoritative and uses exact nearest sampling.
    if bmp_file.exists() {
        return load_image_file(ctx, edata_n, &bmp_file, TextureOptions::NEAREST);
    }

    None
}

/// Decode an image file (BMP or PNG) and register it as an egui texture.
#[cfg(not(target_arch = "wasm32"))]
fn load_image_file(
    ctx: &egui::Context,
    edata_n: u16,
    path: &Path,
    texture_options: TextureOptions,
) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;

    load_image_bytes(ctx, edata_n, &bytes, texture_options)
}

fn load_image_bytes(
    ctx: &egui::Context,
    edata_n: u16,
    bytes: &[u8],
    texture_options: TextureOptions,
) -> Option<TextureHandle> {
    // image crate auto-detects format from magic bytes.
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());

    let handle = ctx.load_texture(format!("edata_{edata_n}"), color_image, texture_options);

    Some(handle)
}

#[cfg(target_arch = "wasm32")]
fn load_edata_texture(
    ctx: &egui::Context,
    edata_n: u16,
    _hd_path: Option<&Path>,
    _edata_path: Option<&Path>,
    _profile: AssetRenderProfile,
    _approved_hd: Option<&ApprovedHdAsset>,
) -> Option<TextureHandle> {
    let bytes = wasm_edata_bytes(edata_n)?;
    load_image_bytes(ctx, edata_n, &bytes, TextureOptions::NEAREST)
}

// ---------------------------------------------------------------------------
// Stat display helpers
// ---------------------------------------------------------------------------

/// Two-column stat row: label on left, value on right.
fn stat_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{label}:"))
                .small()
                .color(Color32::from_gray(160)),
        );
        ui.label(RichText::new(value).small());
    });
}

/// Two-column stat row for `SkillPair` values (base ± variance).
fn stat_row_pair(ui: &mut egui::Ui, label: &str, base: u32, variance: u32) {
    let value = if variance > 0 {
        format!("{base} ± {variance}")
    } else {
        base.to_string()
    };
    stat_row(ui, label, &value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_indexed_edata() -> Vec<u8> {
        const WIDTH: usize = 400;
        const HEIGHT: usize = 200;
        const PIXEL_OFFSET: usize = 1_078;
        let mut bytes = vec![0_u8; PIXEL_OFFSET + WIDTH * HEIGHT];
        let file_size = bytes.len() as u32;
        bytes[0..2].copy_from_slice(b"BM");
        bytes[2..6].copy_from_slice(&file_size.to_le_bytes());
        bytes[10..14].copy_from_slice(&(PIXEL_OFFSET as u32).to_le_bytes());
        bytes[14..18].copy_from_slice(&40_u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&(WIDTH as i32).to_le_bytes());
        bytes[22..26].copy_from_slice(&(HEIGHT as i32).to_le_bytes());
        bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
        bytes[28..30].copy_from_slice(&8_u16.to_le_bytes());
        bytes[34..38].copy_from_slice(&((WIDTH * HEIGHT) as u32).to_le_bytes());
        bytes[46..50].copy_from_slice(&256_u32.to_le_bytes());
        bytes[54 + 4..54 + 8].copy_from_slice(&[0x20, 0x80, 0xe0, 0]);
        bytes[PIXEL_OFFSET..].fill(1);
        bytes
    }

    #[test]
    fn encyclopedia_defaults_to_original_parity() {
        let state = EncyclopediaState::new();
        assert_eq!(state.asset_profile, AssetRenderProfile::OriginalParity);
    }

    #[test]
    fn changing_asset_profile_invalidates_cached_images() {
        let mut state = EncyclopediaState::new();
        state.textures.insert(42, None);

        state.set_asset_profile(AssetRenderProfile::FaithfulHd);

        assert!(state.textures.is_empty());
        assert_eq!(state.asset_profile, AssetRenderProfile::FaithfulHd);
    }

    #[test]
    fn edata_filenames_preserve_the_original_three_digit_identity() {
        assert_eq!(edata_filename(1), "EDATA.001");
        assert_eq!(edata_filename(42), "EDATA.042");
        assert_eq!(edata_filename(192), "EDATA.192");
    }

    #[test]
    fn original_edata_bytes_decode_at_source_size() {
        let ctx = egui::Context::default();
        let texture = load_image_bytes(
            &ctx,
            42,
            &synthetic_indexed_edata(),
            TextureOptions::NEAREST,
        )
        .expect("synthetic original EDATA should decode");

        assert_eq!(texture.size(), [400, 200]);
    }

    #[test]
    fn encyclopedia_category_geometry_matches_the_recovered_native_controls() {
        let controls = encyclopedia_control_specs(CockpitFaction::Alliance);

        assert_eq!(
            controls.map(|control| (control.command_id, control.x, control.width)),
            [
                (0x79, 0, 36),
                (0x7a, 38, 36),
                (0x7b, 76, 36),
                (0x7c, 114, 35),
                (0x7d, 151, 36),
                (0x7e, 189, 36),
                (0x7f, 227, 34),
                (0x80, 263, 36),
                (0x81, 301, 35),
                (0x82, 338, 37),
            ]
        );
        assert_eq!(
            controls.last().unwrap().x + controls.last().unwrap().width,
            375
        );
    }

    #[test]
    fn encyclopedia_category_resources_preserve_faction_variants() {
        let alliance = encyclopedia_control_specs(CockpitFaction::Alliance);
        let empire = encyclopedia_control_specs(CockpitFaction::Empire);

        assert_eq!(
            alliance.map(|control| (control.normal_resource, control.pressed_resource)),
            [
                (10_830, 10_831),
                (10_832, 10_833),
                (10_840, 10_841),
                (10_842, 10_843),
                (10_836, 10_837),
                (10_852, 10_853),
                (10_856, 10_857),
                (10_854, 10_855),
                (10_850, 10_851),
                (10_846, 10_847),
            ]
        );
        assert_eq!(
            empire.map(|control| (control.normal_resource, control.pressed_resource)),
            [
                (10_830, 10_831),
                (10_834, 10_835),
                (10_838, 10_839),
                (10_844, 10_845),
                (10_836, 10_837),
                (10_852, 10_853),
                (10_856, 10_857),
                (10_854, 10_855),
                (10_850, 10_851),
                (10_848, 10_849),
            ]
        );
    }

    #[test]
    fn encyclopedia_hit_testing_excludes_right_and_bottom_edges() {
        let rect = egui::Rect::from_min_max(egui::pos2(22.0, 46.0), egui::pos2(58.0, 87.0));

        assert!(encyclopedia_rect_contains(rect, egui::pos2(22.0, 46.0)));
        assert!(encyclopedia_rect_contains(rect, egui::pos2(57.999, 86.999)));
        assert!(!encyclopedia_rect_contains(rect, egui::pos2(58.0, 46.0)));
        assert!(!encyclopedia_rect_contains(rect, egui::pos2(22.0, 87.0)));
    }
}
