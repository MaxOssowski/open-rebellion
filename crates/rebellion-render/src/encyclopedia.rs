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
    /// Selected original index category command (`0x6f..=0x75`).
    ///
    /// The replacement panel does not consume this field. It is retained for
    /// the source-exact index renderer that will replace that panel.
    pub original_category_command: u16,
    /// Stable compound object identity selected in the original index.
    pub original_selected_object_id: Option<u32>,
    /// The original index list's first visible pixel (the list scrolls by
    /// pixels, `FUN_0060a490`).
    pub original_scroll: usize,
    /// A press held on the index list's scroll bar.
    pub original_scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// The original window shows the selected object's topic (command
    /// `0x67`) instead of the index (`0x68`, `FUN_0045f480`).
    pub original_topic_mode: bool,
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
            original_category_command: 0x6f,
            original_selected_object_id: None,
            original_scroll: 0,
            original_scroll_hold: None,
            original_topic_mode: false,
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
// Original index shell
// ---------------------------------------------------------------------------

/// Native Galactic Encyclopedia window dimensions from `FUN_00429f30`.
pub const ENCYCLOPEDIA_INDEX_WIDTH: f32 = 470.0;
pub const ENCYCLOPEDIA_INDEX_HEIGHT: f32 = 330.0;

/// The list (`FUN_0045ddc0`: `FUN_00607ea0(.., 0x24, 0x89, 0x15e, 0xa0, ..,
/// 0x15e, 0x14, .., 10, ..)`): cells 350 by 20 in a 160-pixel list, so
/// eight rows; the text at x 5 (`+0xe8`), centred on the row (`+0xe4 =
/// 0x24`), in font 10.
const ORIGINAL_INDEX_VISIBLE_ROWS: usize = 8;
const ORIGINAL_INDEX_ROW_HEIGHT: f32 = 20.0;
const ORIGINAL_INDEX_TEXT_X: f32 = 5.0;
const ORIGINAL_INDEX_ROW_PIXELS: usize = 20;
const ORIGINAL_INDEX_LIST_PIXELS: usize = ORIGINAL_INDEX_VISIBLE_ROWS * ORIGINAL_INDEX_ROW_PIXELS;
/// The list's arrow step, `FUN_00607ea0`'s argument 10, as the Battle
/// Results' list takes it; its bar is RCDATA 10653 (`FUN_0045ddc0`:
/// `FUN_0060a490(.., list, 7, 0x299d, ..)`), the Battle Results' art.
const ORIGINAL_INDEX_STEP: f32 = 10.0;
/// The list's colours from its constructor (`FUN_00607ea0`: `+0xd8 =
/// 0x787878`, `+0xdc = 0xffffff`): grey rows, the selected one white, with
/// no other mark (A0 2026-10-29).
const ORIGINAL_INDEX_ROW_COLOR: Color32 = Color32::from_rgb(0x78, 0x78, 0x78);

/// One immutable source object shown by the authentic index list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalEncyclopediaEntry {
    pub object_id: u32,
    pub name: String,
}

impl OriginalEncyclopediaEntry {
    #[must_use]
    pub const fn family(&self) -> u8 {
        (self.object_id >> 24) as u8
    }
}

/// Localized source catalog transported from the original DAT/TEXTSTRA data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalEncyclopediaCatalog {
    pub title: String,
    pub topic_label: String,
    category_labels: [String; 7],
    entries: Vec<OriginalEncyclopediaEntry>,
    /// The game's systems, when one runs: `FUN_00422620` lists the systems
    /// of the viewer's side (`FUN_004f31b0`), so a smaller galaxy's index
    /// leaves the others out.
    systems: Option<std::collections::BTreeSet<u32>>,
}

impl OriginalEncyclopediaCatalog {
    #[must_use]
    pub fn new(
        title: String,
        topic_label: String,
        category_labels: [String; 7],
        mut entries: Vec<OriginalEncyclopediaEntry>,
    ) -> Self {
        entries.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.object_id.cmp(&right.object_id))
        });
        entries.dedup_by_key(|entry| entry.object_id);
        Self {
            title,
            topic_label,
            category_labels,
            entries,
            systems: None,
        }
    }

    /// Limit the System entries to the game's systems, by their object id.
    pub fn set_galaxy_systems(&mut self, systems: impl IntoIterator<Item = u32>) {
        self.systems = Some(systems.into_iter().collect());
    }

    #[must_use]
    pub fn category_label(&self, command_id: u16) -> Option<&str> {
        let index = usize::from(command_id.checked_sub(0x6f)?);
        self.category_labels.get(index).map(String::as_str)
    }

    #[must_use]
    pub fn entries_for(&self, command_id: u16) -> Vec<&OriginalEncyclopediaEntry> {
        if !(0x6f..=0x75).contains(&command_id) {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|entry| original_category_contains(command_id, entry.family()))
            .filter(|entry| {
                !(0x90..0x98).contains(&entry.family())
                    || self
                        .systems
                        .as_ref()
                        .is_none_or(|systems| systems.contains(&entry.object_id))
            })
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

const fn original_category_contains(command_id: u16, family: u8) -> bool {
    match command_id {
        0x6f => true,
        0x70 => family >= 0x90 && family < 0x98,
        0x71 => family >= 0x14 && family < 0x20,
        0x72 => family >= 0x20 && family < 0x30,
        0x73 => family >= 0x40 && family < 0x80,
        0x74 => family >= 0x10 && family < 0x14,
        0x75 => family >= 0x30 && family < 0x40,
        _ => false,
    }
}

const INDEX_CONTENT: u32 = 10_338;
const INDEX_CONTENT_X: f32 = 12.0;
const INDEX_CONTENT_Y: f32 = 13.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OriginalControlSpec {
    command_id: u16,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    normal_resource: u32,
    pressed_resource: u32,
    selected_in_index: bool,
}

const fn encyclopedia_category_controls(
    faction: crate::cockpit::CockpitFaction,
) -> [OriginalControlSpec; 7] {
    let (ships_normal, ships_pressed, facilities_normal, facilities_pressed) = match faction {
        crate::cockpit::CockpitFaction::Alliance => (10_348, 10_347, 10_344, 10_343),
        crate::cockpit::CockpitFaction::Empire => (10_360, 10_359, 10_356, 10_355),
    };
    let (personnel_normal, personnel_pressed, missions_normal, missions_pressed) = match faction {
        crate::cockpit::CockpitFaction::Alliance => (10_346, 10_345, 11_616, 11_615),
        crate::cockpit::CockpitFaction::Empire => (10_358, 10_357, 11_618, 11_617),
    };
    let (troops_normal, troops_pressed) = match faction {
        crate::cockpit::CockpitFaction::Alliance => (10_352, 10_351),
        crate::cockpit::CockpitFaction::Empire => (10_362, 10_361),
    };
    [
        OriginalControlSpec {
            command_id: 0x6f,
            x: 36,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: 10_340,
            pressed_resource: 10_339,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x70,
            x: 88,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: 10_350,
            pressed_resource: 10_349,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x71,
            x: 140,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: ships_normal,
            pressed_resource: ships_pressed,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x72,
            x: 192,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: facilities_normal,
            pressed_resource: facilities_pressed,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x73,
            x: 244,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: missions_normal,
            pressed_resource: missions_pressed,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x74,
            x: 296,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: troops_normal,
            pressed_resource: troops_pressed,
            selected_in_index: false,
        },
        OriginalControlSpec {
            command_id: 0x75,
            x: 348,
            y: 78,
            width: 49,
            height: 41,
            normal_resource: personnel_normal,
            pressed_resource: personnel_pressed,
            selected_in_index: false,
        },
    ]
}

const fn encyclopedia_rail_controls(
    faction: crate::cockpit::CockpitFaction,
) -> [OriginalControlSpec; 3] {
    match faction {
        crate::cockpit::CockpitFaction::Alliance => [
            OriginalControlSpec {
                command_id: 0xfb,
                x: 423,
                y: 25,
                width: 32,
                height: 31,
                normal_resource: 10_370,
                pressed_resource: 10_371,
                selected_in_index: false,
            },
            OriginalControlSpec {
                command_id: 0x67,
                x: 423,
                y: 93,
                width: 32,
                height: 31,
                normal_resource: 10_374,
                pressed_resource: 10_375,
                selected_in_index: false,
            },
            OriginalControlSpec {
                command_id: 0x68,
                x: 423,
                y: 147,
                width: 32,
                height: 31,
                normal_resource: 10_372,
                pressed_resource: 10_373,
                selected_in_index: true,
            },
        ],
        crate::cockpit::CockpitFaction::Empire => [
            OriginalControlSpec {
                command_id: 0xfb,
                x: 426,
                y: 21,
                width: 44,
                height: 41,
                normal_resource: 10_376,
                pressed_resource: 10_377,
                selected_in_index: false,
            },
            OriginalControlSpec {
                command_id: 0x67,
                x: 426,
                y: 89,
                width: 44,
                height: 41,
                normal_resource: 10_380,
                pressed_resource: 10_381,
                selected_in_index: false,
            },
            OriginalControlSpec {
                command_id: 0x68,
                x: 426,
                y: 143,
                width: 44,
                height: 41,
                normal_resource: 10_378,
                pressed_resource: 10_379,
                selected_in_index: true,
            },
        ],
    }
}

/// Paint the source-exact Galactic Encyclopedia index bitmap layers.
///
/// Text labels and the object list are intentionally absent until their
/// `TEXTSTRA` and native-list contracts are transported. Category resource
/// `0x75` is painted at native size and clipped to its 49-by-41 control, as
/// the original constructor does; it is never scaled from 57 pixels high.
pub fn draw_encyclopedia_index_shell(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: crate::cockpit::CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    selected_category: u16,
) -> Option<u16> {
    draw_encyclopedia_index_layers(ctx, cache, faction, origin, scale, selected_category, None)
        .activated_command
}

/// Paint the authentic index shell together with localized source rows.
///
/// The catalog is immutable reference data and selection is stored by compound
/// object id, so category changes and source reordering cannot silently select
/// a different object. Topic composition remains a later parity checkpoint.
pub fn draw_encyclopedia_index_catalog(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: crate::cockpit::CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
) -> Option<u16> {
    if scale <= 0.0 || catalog.is_empty() {
        return None;
    }
    reconcile_original_index_state(state, catalog, false);
    let entries = catalog.entries_for(state.original_category_command);
    let selected_name = entries
        .iter()
        .find(|entry| Some(entry.object_id) == state.original_selected_object_id)
        .map(|entry| entry.name.as_str())
        .unwrap_or("");
    let category_label = catalog
        .category_label(state.original_category_command)
        .unwrap_or("");
    let content = OriginalIndexContent {
        title: &catalog.title,
        topic_label: &catalog.topic_label,
        category_label,
        selected_name,
        entries: &entries,
        selected_object_id: state.original_selected_object_id,
        scroll: state.original_scroll,
        scroll_hold: state.original_scroll_hold,
    };
    let outcome = draw_encyclopedia_index_layers(
        ctx,
        cache,
        faction,
        origin,
        scale,
        state.original_category_command,
        Some(content),
    );
    if let Some(command) = outcome.activated_command {
        if (0x6f..=0x75).contains(&command) && command != state.original_category_command {
            // FUN_0045f100 rebuilds the list from its top.
            state.original_category_command = command;
            state.original_scroll = 0;
            reconcile_original_index_state(state, catalog, true);
        }
        // FUN_0045f480: the rail's topic command shows the selected object.
        if command == 0x67 && state.original_selected_object_id.is_some() {
            state.original_topic_mode = true;
        }
    }
    state.original_scroll_hold = outcome.scroll_hold;
    if let Some(scroll) = outcome.scroll {
        state.original_scroll = scroll;
        reconcile_original_index_state(state, catalog, false);
    }
    if let Some(object_id) = outcome.selected_object_id {
        state.original_selected_object_id = Some(object_id);
        reconcile_original_index_state(state, catalog, true);
    }
    outcome.activated_command
}

/// The Encyclopedia object a menu object opens at (`FUN_0045d400` resolves
/// its initial object, else the index): its class's or its own DAT id,
/// the catalog's compound id. A fleet or a production manager has none.
/// port: a system icon opens its system.
#[must_use]
pub fn original_encyclopedia_object(
    world: &GameWorld,
    object: crate::object_menu::MenuObject,
) -> Option<u32> {
    use crate::object_menu::MenuObject;
    let dat_id = match object {
        MenuObject::Character(key) => world.characters.get(key)?.dat_id,
        MenuObject::SpecialForce(key) => world.special_forces.get(key)?.class_dat_id,
        MenuObject::Troop(key) => world.troops.get(key)?.class_dat_id,
        MenuObject::Ship { fleet, index, .. } => {
            let ship = world.fleets.get(fleet)?.capital_ships.get(index)?;
            world.capital_ship_classes.get(ship.class)?.dat_id
        }
        MenuObject::Fighter { fleet, index } => {
            let entry = world.fleets.get(fleet)?.fighters.get(index)?;
            world.fighter_classes.get(entry.class)?.dat_id
        }
        MenuObject::PlanetFighter { class, .. } => world.fighter_classes.get(class)?.dat_id,
        MenuObject::DefenseFacility(key) => world.defense_facilities.get(key)?.class_dat_id,
        MenuObject::ManufacturingFacility(key) => {
            world.manufacturing_facilities.get(key)?.class_dat_id
        }
        MenuObject::ProductionFacility(key) => world.production_facilities.get(key)?.class_dat_id,
        MenuObject::SystemIcon { system, .. } => world.systems.get(system)?.dat_id,
        MenuObject::Fleet(_) | MenuObject::Producer { .. } => return None,
    };
    Some(dat_id.raw())
}

/// Open the original window at `object`'s topic when the catalog lists it
/// (All Databases, so the topic steps through every object), else at the
/// index. Each opening builds a new window (`FUN_0045d400`), whose list
/// starts at All Databases with nothing selected (`FUN_0045f100(0x6f, 1)`
/// clears `+0x148`).
pub fn open_original_encyclopedia_at(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    object: Option<u32>,
) {
    state.open = true;
    state.original_topic_mode = false;
    state.original_category_command = 0x6f;
    state.original_selected_object_id = None;
    state.original_scroll = 0;
    if let Some(object) = object.filter(|object| {
        catalog
            .entries_for(0x6f)
            .iter()
            .any(|entry| entry.object_id == *object)
    }) {
        state.original_category_command = 0x6f;
        state.original_selected_object_id = Some(object);
        state.original_topic_mode = true;
        reconcile_original_index_state(state, catalog, true);
    }
}

/// One object's topic, from the source bindings
/// (`rebellion_data::encyclopedia_topics`, `FUN_0045fa60`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalEncyclopediaTopic {
    pub title: String,
    /// `ENCYTEXT.DLL` text; `None` shows an empty body.
    pub body: Option<String>,
    /// The `EDATA.NNN` number of its artwork.
    pub edata: Option<u16>,
}

impl OriginalEncyclopediaTopic {
    /// The `EDATA.NNN` number in an artwork file name such as `EDATA.014`.
    #[must_use]
    pub fn edata_number(filename: &str) -> Option<u16> {
        filename
            .strip_prefix("EDATA.")
            .or_else(|| filename.strip_prefix("edata."))?
            .parse()
            .ok()
    }
}

/// The selected object's neighbours in the current category's list:
/// previous and next (`FUN_0045fd20` enables `0x83` / `0x84` when the list
/// item has a previous / next item).
#[must_use]
pub fn original_topic_neighbours(
    state: &EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
) -> (Option<u32>, Option<u32>) {
    let entries = catalog.entries_for(state.original_category_command);
    let Some(index) = entries
        .iter()
        .position(|entry| Some(entry.object_id) == state.original_selected_object_id)
    else {
        return (None, None);
    };
    let previous = index
        .checked_sub(1)
        .and_then(|before| entries.get(before))
        .map(|entry| entry.object_id);
    let next = entries.get(index + 1).map(|entry| entry.object_id);
    (previous, next)
}

/// Apply a topic command: `0x83` / `0x84` step to the previous / next
/// object (`FUN_0045fe60` gives Left and Right the same), `0x68` returns to
/// the index (`FUN_0045f480`).
pub fn apply_original_topic_command(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    command: u16,
) {
    let (previous, next) = original_topic_neighbours(state, catalog);
    match command {
        0x83 => {
            if let Some(object) = previous {
                state.original_selected_object_id = Some(object);
            }
        }
        0x84 => {
            if let Some(object) = next {
                state.original_selected_object_id = Some(object);
            }
        }
        0x68 => state.original_topic_mode = false,
        _ => {}
    }
    reconcile_original_index_state(state, catalog, true);
}

const TOPIC_CONTENT: u32 = 10_337;
const TOPIC_PREVIOUS: [u32; 3] = [10_385, 10_386, 10_387];
const TOPIC_NEXT: [u32; 3] = [10_382, 10_383, 10_384];

/// Paint the original window's topic mode (`FUN_0045ddc0`): the faction
/// base and rail with Topic current, STRATEGY 10337 at (12, 14), the
/// object's 400 x 200 EDATA picture at (12, 31), its title at (36, 14) in
/// a 350 wide box, previous `0x83` at (28, 14) and next `0x84` at
/// (380, 14) (21 x 17, the third resource when unavailable), and the body
/// in (17, 231, 395, 80). Returns the command activated.
#[expect(
    clippy::too_many_arguments,
    reason = "The window's context, art cache, side, placement, state, catalog and topic."
)]
pub fn draw_encyclopedia_topic(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: crate::cockpit::CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    topic: &OriginalEncyclopediaTopic,
) -> Option<u16> {
    if scale <= 0.0 {
        return None;
    }
    let (previous, next) = original_topic_neighbours(state, catalog);
    let picture = topic
        .edata
        .and_then(|edata| state.artwork_texture(ctx, edata));
    let mut activated = None;
    egui::Area::new(egui::Id::new("original-encyclopedia-topic"))
        .fixed_pos(origin)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let size = egui::vec2(
                ENCYCLOPEDIA_INDEX_WIDTH * scale,
                ENCYCLOPEDIA_INDEX_HEIGHT * scale,
            );
            let (window_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let (base, rail) = match faction {
                crate::cockpit::CockpitFaction::Alliance => (10_335, 10_585),
                crate::cockpit::CockpitFaction::Empire => (10_336, 10_589),
            };
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                base,
                window_rect.min,
                scale,
                window_rect,
            );
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                rail,
                encyclopedia_point(window_rect, scale, 412.0, 0.0),
                scale,
                window_rect,
            );
            let content_rect = encyclopedia_rect(window_rect, scale, 12.0, 14.0, 400.0, 306.0);
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                TOPIC_CONTENT,
                content_rect.min,
                scale,
                content_rect,
            );
            if let Some(texture) = picture {
                let rect = encyclopedia_rect(window_rect, scale, 12.0, 31.0, 400.0, 200.0);
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            for control in encyclopedia_rail_controls(faction) {
                // Topic mode makes 0x67 current.
                if draw_original_control(
                    ui,
                    ctx,
                    cache,
                    window_rect,
                    scale,
                    control,
                    control.command_id == 0x67,
                ) {
                    activated = Some(control.command_id);
                }
            }
            for (command, x, resources, available) in [
                (0x83_u16, 28_u16, TOPIC_PREVIOUS, previous.is_some()),
                (0x84, 380, TOPIC_NEXT, next.is_some()),
            ] {
                let control = OriginalControlSpec {
                    command_id: command,
                    x,
                    y: 14,
                    width: 21,
                    height: 17,
                    normal_resource: resources[0],
                    pressed_resource: resources[1],
                    selected_in_index: false,
                };
                if available {
                    if draw_original_control(ui, ctx, cache, window_rect, scale, control, false) {
                        activated = Some(command);
                    }
                } else {
                    let rect =
                        encyclopedia_rect(window_rect, scale, f32::from(x), 14.0, 21.0, 17.0);
                    paint_original_resource_native(
                        ui.painter(),
                        ctx,
                        cache,
                        resources[2],
                        rect.min,
                        scale,
                        rect,
                    );
                }
            }
            // The title label +0x120 (FUN_0045fa60's FUN_00601aa0), font 5,
            // as in the index (A0 2026-10-29: bold).
            let title_font = crate::theme::original_font(5, scale);
            ui.painter()
                .with_clip_rect(encyclopedia_rect(
                    window_rect,
                    scale,
                    36.0,
                    14.0,
                    350.0,
                    17.0,
                ))
                .text(
                    encyclopedia_point(window_rect, scale, 211.0, 14.0),
                    egui::Align2::CENTER_TOP,
                    &topic.title,
                    title_font,
                    Color32::WHITE,
                );
            if let Some(body) = &topic.body {
                let body_rect = encyclopedia_rect(window_rect, scale, 17.0, 231.0, 395.0, 80.0);
                // hyp: the body's text object (FUN_0041ecf0) font is not
                // read. A0 2026-10-29 measures it: lines 16 apart from
                // (17, 234), the first line 374 wide, which Arial's metrics
                // give at a 13-pixel em (Liberation Sans: 372).
                let mut job = egui::text::LayoutJob::single_section(
                    body.clone(),
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(13.0 * scale),
                        color: Color32::WHITE,
                        line_height: Some(16.0 * scale),
                        ..egui::TextFormat::default()
                    },
                );
                job.wrap.max_width = body_rect.width();
                let galley = ui.fonts(|fonts| fonts.layout_job(job));
                ui.painter().with_clip_rect(body_rect).galley(
                    body_rect.min,
                    galley,
                    Color32::WHITE,
                );
            }
        });
    // FUN_0045fe60: Left and Right step through the topics.
    if activated.is_none() {
        activated = ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) && previous.is_some() {
                Some(0x83)
            } else if input.key_pressed(egui::Key::ArrowRight) && next.is_some() {
                Some(0x84)
            } else {
                None
            }
        });
    }
    if let Some(command) = activated {
        apply_original_topic_command(state, catalog, command);
    }
    activated
}

fn reconcile_original_index_state(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    reveal_selection: bool,
) {
    if !(0x6f..=0x75).contains(&state.original_category_command) {
        state.original_category_command = 0x6f;
    }
    let entries = catalog.entries_for(state.original_category_command);
    if state.original_selected_object_id.is_some_and(|selected| {
        !entries.iter().any(|entry| entry.object_id == selected)
    }) {
        // FUN_0045f100 rebuilds the list with no item selected, from its
        // top. A list with nothing selected keeps its scroll.
        state.original_selected_object_id = None;
        state.original_scroll = 0;
    }
    let selected_index = entries
        .iter()
        .position(|entry| Some(entry.object_id) == state.original_selected_object_id)
        .unwrap_or(0);
    let max_scroll = original_index_max_scroll(entries.len());
    state.original_scroll = state.original_scroll.min(max_scroll);
    if reveal_selection {
        // The selected row's pixels, brought into the 160-pixel list.
        let top = selected_index * ORIGINAL_INDEX_ROW_PIXELS;
        let bottom = top + ORIGINAL_INDEX_ROW_PIXELS;
        if top < state.original_scroll {
            state.original_scroll = top;
        } else if bottom > state.original_scroll + ORIGINAL_INDEX_LIST_PIXELS {
            state.original_scroll = (bottom - ORIGINAL_INDEX_LIST_PIXELS).min(max_scroll);
        }
    }
}

/// The list's scroll range for `rows` entries, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 20 pixels."
)]
fn original_index_max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first(
        (rows * ORIGINAL_INDEX_ROW_PIXELS) as f32,
        ORIGINAL_INDEX_LIST_PIXELS as f32,
    ) as usize
}

#[derive(Default)]
struct OriginalIndexOutcome {
    activated_command: Option<u16>,
    selected_object_id: Option<u32>,
    scroll: Option<usize>,
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
}

struct OriginalIndexContent<'a> {
    title: &'a str,
    topic_label: &'a str,
    category_label: &'a str,
    selected_name: &'a str,
    entries: &'a [&'a OriginalEncyclopediaEntry],
    selected_object_id: Option<u32>,
    scroll: usize,
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
}

fn draw_encyclopedia_index_layers(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: crate::cockpit::CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    selected_category: u16,
    content: Option<OriginalIndexContent<'_>>,
) -> OriginalIndexOutcome {
    if scale <= 0.0 || !(0x6f..=0x75).contains(&selected_category) {
        return OriginalIndexOutcome::default();
    }

    let mut outcome = OriginalIndexOutcome::default();
    egui::Area::new(egui::Id::new("original-encyclopedia-index-shell"))
        .fixed_pos(origin)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let size = egui::vec2(
                ENCYCLOPEDIA_INDEX_WIDTH * scale,
                ENCYCLOPEDIA_INDEX_HEIGHT * scale,
            );
            let (window_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let (base, rail) = match faction {
                crate::cockpit::CockpitFaction::Alliance => (10_335, 10_585),
                crate::cockpit::CockpitFaction::Empire => (10_336, 10_589),
            };
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                base,
                window_rect.min,
                scale,
                window_rect,
            );
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                rail,
                encyclopedia_point(window_rect, scale, 412.0, 0.0),
                scale,
                window_rect,
            );
            let content_rect = encyclopedia_rect(
                window_rect,
                scale,
                INDEX_CONTENT_X,
                INDEX_CONTENT_Y,
                400.0,
                306.0,
            );
            paint_original_resource_native(
                ui.painter(),
                ctx,
                cache,
                INDEX_CONTENT,
                content_rect.min,
                scale,
                content_rect,
            );

            for control in encyclopedia_category_controls(faction) {
                if draw_original_control(
                    ui,
                    ctx,
                    cache,
                    window_rect,
                    scale,
                    control,
                    control.command_id == selected_category,
                ) {
                    outcome.activated_command = Some(control.command_id);
                }
            }
            for control in encyclopedia_rail_controls(faction) {
                if draw_original_control(
                    ui,
                    ctx,
                    cache,
                    window_rect,
                    scale,
                    control,
                    control.selected_in_index,
                ) {
                    outcome.activated_command = Some(control.command_id);
                }
            }
            if let Some(content) = content {
                draw_original_index_content(ui, cache, window_rect, scale, content, &mut outcome);
            }
        });
    outcome
}

fn draw_original_index_content(
    ui: &mut egui::Ui,
    cache: &mut BmpCache,
    window_rect: egui::Rect,
    scale: f32,
    content: OriginalIndexContent<'_>,
    outcome: &mut OriginalIndexOutcome,
) {
    let painter = ui.painter();
    // FUN_0045ddc0: the title TEXTSTRA 0x1842 in font 5, centred in
    // (36, 14, 350); "Topic" 0x1843 at (36, 48) in font 4; the category
    // label at (40, 119) in font 5; the list in font 10. hyp: the
    // selector's font 100 (FUN_00604cf0, +0x24) is not mapped; font 10
    // stands in.
    let title_font = crate::theme::original_font(5, scale);
    let topic_font = crate::theme::original_font(4, scale);
    let list_font = crate::theme::original_font(10, scale);
    painter.text(
        encyclopedia_point(window_rect, scale, 211.0, 14.0),
        egui::Align2::CENTER_TOP,
        content.title,
        title_font.clone(),
        Color32::WHITE,
    );
    painter.text(
        encyclopedia_point(window_rect, scale, 36.0, 48.0),
        egui::Align2::LEFT_TOP,
        content.topic_label,
        topic_font,
        Color32::WHITE,
    );
    painter.text(
        encyclopedia_point(window_rect, scale, 143.0, 47.0),
        egui::Align2::LEFT_TOP,
        content.selected_name,
        list_font.clone(),
        Color32::WHITE,
    );
    painter.text(
        encyclopedia_point(window_rect, scale, 40.0, 119.0),
        egui::Align2::LEFT_TOP,
        content.category_label,
        title_font,
        Color32::WHITE,
    );

    let list_rect = original_index_list_rect(window_rect, scale);
    let list_painter = painter.with_clip_rect(list_rect);
    #[expect(
        clippy::cast_precision_loss,
        reason = "A few hundred rows of 20 pixels."
    )]
    let rows_height = (content.entries.len() * ORIGINAL_INDEX_ROW_PIXELS) as f32;
    let bar_at = |first: f32| {
        crate::list_scroll_bar::ScrollBar::new(
            crate::list_scroll_bar::RESULTS_ART,
            36.0,
            137.0,
            350.0,
            160.0,
            rows_height,
            first,
        )
    };
    // The bar: the pointer's press, hold and release on it, then the wheel.
    let (pointer, pressed, down, released, now) = ui.input(|input| {
        (
            input.pointer.interact_pos(),
            input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary),
            input.time,
        )
    });
    let at = pointer.map_or((-1.0, -1.0), |point| {
        (
            (point.x - window_rect.min.x) / scale,
            (point.y - window_rect.min.y) / scale,
        )
    });
    #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
    let bar = bar_at(content.scroll as f32);
    let mut hold = content.scroll_hold;
    let mut first = crate::list_scroll_bar::update(
        &bar,
        &mut hold,
        crate::list_scroll_bar::Pointer {
            at,
            pressed,
            down,
            released,
        },
        ORIGINAL_INDEX_STEP,
        now,
    );
    // port: the original has no wheel; a notch moves a row.
    if ui.rect_contains_pointer(list_rect) {
        let wheel = ui.input(|input| input.raw_scroll_delta.y);
        if wheel.abs() > f32::EPSILON {
            #[expect(clippy::cast_precision_loss, reason = "A row's pixels.")]
            let row = ORIGINAL_INDEX_ROW_PIXELS as f32;
            first = (first - wheel.signum() * row).clamp(0.0, bar.max);
        }
    }
    outcome.scroll_hold = hold;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Within 0..=max."
    )]
    let scroll = first.max(0.0) as usize;
    if scroll != content.scroll {
        outcome.scroll = Some(scroll);
    }

    #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
    let offset = scroll as f32;
    for (index, entry) in content.entries.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "A few hundred rows.")]
        let y = 137.0 + index as f32 * ORIGINAL_INDEX_ROW_HEIGHT - offset;
        if y + ORIGINAL_INDEX_ROW_HEIGHT <= 137.0 || y >= 297.0 {
            continue;
        }
        let row_rect = original_index_row_rect(window_rect, scale, y);
        let selected = Some(entry.object_id) == content.selected_object_id;
        let response = ui.interact(
            row_rect,
            ui.id().with(("encyclopedia-index-row", entry.object_id)),
            egui::Sense::click(),
        );
        list_painter.text(
            encyclopedia_point(
                window_rect,
                scale,
                36.0 + ORIGINAL_INDEX_TEXT_X,
                y + ORIGINAL_INDEX_ROW_HEIGHT / 2.0,
            ),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            list_font.clone(),
            if selected {
                Color32::WHITE
            } else {
                ORIGINAL_INDEX_ROW_COLOR
            },
        );
        if response.clicked() {
            outcome.selected_object_id = Some(entry.object_id);
        }
    }

    // The held arrow draws pressed while the pointer stays on it.
    let bar = bar_at(offset);
    let held = hold.map(|hold| hold.part).filter(|part| {
        matches!(
            part,
            crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down
        ) && down
            && bar.part_at(at) == Some(*part)
    });
    let (bar_x, bar_y, bar_w, bar_h) = bar.rect();
    let bar_painter = painter.with_clip_rect(encyclopedia_rect(
        window_rect,
        scale,
        bar_x,
        bar_y,
        bar_w,
        bar_h,
    ));
    for piece in bar.pieces(held) {
        crate::fleet_window::paint_native(
            &bar_painter,
            ui.ctx(),
            cache,
            DllSource::Strategy,
            piece.id,
            window_rect,
            scale,
            piece.x,
            piece.y,
        );
    }
}

fn draw_original_control(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    window_rect: egui::Rect,
    scale: f32,
    control: OriginalControlSpec,
    selected: bool,
) -> bool {
    let rect = encyclopedia_rect(
        window_rect,
        scale,
        f32::from(control.x),
        f32::from(control.y),
        f32::from(control.width),
        f32::from(control.height),
    );
    let control_id = ui
        .id()
        .with(("encyclopedia-original-control", control.command_id));
    let response = ui.interact(rect, control_id, egui::Sense::click());
    let capture_id = control_id.with("opaque-press-origin");
    if ctx.input(|input| input.pointer.primary_pressed()) {
        let captured = ctx.pointer_latest_pos().is_some_and(|point| {
            original_control_contains(cache, control.normal_resource, rect, scale, point)
        });
        ui.data_mut(|data| data.insert_temp(capture_id, captured));
    }
    let captured = ui.data(|data| data.get_temp::<bool>(capture_id).unwrap_or(false));
    let (pointer, primary_down) = ctx.input(|input| {
        (
            input.pointer.interact_pos(),
            input.pointer.button_down(egui::PointerButton::Primary),
        )
    });
    let pointer_hits = pointer.is_some_and(|point| {
        original_control_contains(cache, control.normal_resource, rect, scale, point)
    });
    let pressed = selected
        || (primary_down && response.is_pointer_button_down_on() && captured && pointer_hits);
    paint_original_resource_native(
        ui.painter(),
        ctx,
        cache,
        if pressed {
            control.pressed_resource
        } else {
            control.normal_resource
        },
        rect.min,
        scale,
        rect,
    );
    captured
        && response.clicked()
        && response.interact_pointer_pos().is_some_and(|point| {
            original_control_contains(cache, control.normal_resource, rect, scale, point)
        })
}

fn original_control_contains(
    cache: &mut BmpCache,
    resource_id: u32,
    rect: egui::Rect,
    scale: f32,
    point: egui::Pos2,
) -> bool {
    if point.x < rect.min.x
        || point.x >= rect.max.x
        || point.y < rect.min.y
        || point.y >= rect.max.y
    {
        return false;
    }
    let x = ((point.x - rect.min.x) / scale).floor() as usize;
    let y = ((point.y - rect.min.y) / scale).floor() as usize;
    cache.is_resource_hit(DllSource::Strategy, resource_id, x, y)
}

/// The window's top-left on screen: centred in the galaxy view with
/// truncating halves, as `FUN_00606980` places a window (A0 2026-10-29, the
/// Empire's at (125, 52); captures 281117 and 281123, the Alliance's at
/// (62, 50)). `FUN_0045d400` creates it at (0, 0); hyp: it is then placed
/// as the galaxy view's other windows are.
#[must_use]
pub fn encyclopedia_origin(layout: crate::cockpit::CockpitLayout) -> egui::Pos2 {
    crate::mission_dialog::galaxy_centered_rect(
        layout,
        ENCYCLOPEDIA_INDEX_WIDTH,
        ENCYCLOPEDIA_INDEX_HEIGHT,
    )
    .min
}

fn encyclopedia_point(parent: egui::Rect, scale: f32, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(parent.min.x + x * scale, parent.min.y + y * scale)
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
        encyclopedia_point(parent, scale, x, y),
        egui::vec2(width * scale, height * scale),
    )
}

fn original_index_list_rect(parent: egui::Rect, scale: f32) -> egui::Rect {
    encyclopedia_rect(parent, scale, 36.0, 137.0, 350.0, 160.0)
}

/// A row's click area at its top `y` (window pixels), left of the bar and
/// within the list.
fn original_index_row_rect(parent: egui::Rect, scale: f32, y: f32) -> egui::Rect {
    encyclopedia_rect(
        parent,
        scale,
        36.0,
        y,
        338.0,
        ORIGINAL_INDEX_ROW_HEIGHT,
    )
    .intersect(original_index_list_rect(parent, scale))
}

fn paint_original_resource_native(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    resource_id: u32,
    origin: egui::Pos2,
    scale: f32,
    clip_rect: egui::Rect,
) {
    let Some(texture) = cache.get(ctx, DllSource::Strategy, resource_id) else {
        return;
    };
    let rect = egui::Rect::from_min_size(origin, texture.size_vec2() * scale);
    painter.with_clip_rect(clip_rect).image(
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
    faction: crate::cockpit::CockpitFaction,
    selected_category: u16,
) -> Option<u16> {
    draw_encyclopedia_index_shell(
        ctx,
        cache,
        faction,
        egui::pos2(85.0, 55.0),
        1.0,
        selected_category,
    )
}

/// Fixed-position catalog adapter for deterministic browser inspection.
#[cfg(feature = "interface-test-fixtures")]
pub fn draw_encyclopedia_index_catalog_fixture(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: crate::cockpit::CockpitFaction,
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
) -> Option<u16> {
    draw_encyclopedia_index_catalog(
        ctx,
        cache,
        faction,
        egui::pos2(85.0, 55.0),
        1.0,
        state,
        catalog,
    )
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
    if let Some(handle) = state.artwork_texture(ctx, edata_n) {
        let size = egui::vec2(400.0, 200.0);
        ui.add(egui::Image::from_texture((handle.id(), size)));
    } else {
        show_placeholder_image(ui);
    }
}

impl EncyclopediaState {
    /// The texture of `EDATA.NNN`, loaded on first use and kept. The
    /// Message window shows the same artwork for a message about an
    /// object (`FUN_0045f970`).
    pub fn artwork_texture(&mut self, ctx: &egui::Context, edata_n: u16) -> Option<TextureHandle> {
        // Lazy-load the texture if not yet cached. HD is an explicit
        // enhancement profile; original parity never probes the HD tree.
        if !self.textures.contains_key(&edata_n) {
            let handle = load_edata_texture(
                ctx,
                edata_n,
                self.hd_path.as_deref(),
                self.edata_path.as_deref(),
                self.asset_profile,
                self.approved_hd_assets
                    .get(&format!("edata/EDATA_{edata_n:03}")),
            );
            if handle.is_none() {
                eprintln!(
                    "[encyclopedia] original artwork unavailable asset={}",
                    edata_filename(edata_n)
                );
            }
            self.textures.insert(edata_n, handle);
        }
        self.textures.get(&edata_n).cloned().flatten()
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
    fn original_index_categories_match_the_recovered_command_geometry() {
        let controls = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Alliance);

        assert_eq!(
            controls.map(|control| (
                control.command_id,
                control.x,
                control.y,
                control.width,
                control.height,
            )),
            [
                (0x6f, 36, 78, 49, 41),
                (0x70, 88, 78, 49, 41),
                (0x71, 140, 78, 49, 41),
                (0x72, 192, 78, 49, 41),
                (0x73, 244, 78, 49, 41),
                (0x74, 296, 78, 49, 41),
                (0x75, 348, 78, 49, 41),
            ]
        );
    }

    #[test]
    fn original_index_category_resources_preserve_faction_variants() {
        let alliance = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Alliance);
        let empire = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Empire);

        assert_eq!(
            alliance.map(|control| (control.normal_resource, control.pressed_resource)),
            [
                (10_340, 10_339),
                (10_350, 10_349),
                (10_348, 10_347),
                (10_344, 10_343),
                (11_616, 11_615),
                (10_352, 10_351),
                (10_346, 10_345),
            ]
        );
        assert_eq!(
            empire.map(|control| (control.normal_resource, control.pressed_resource)),
            [
                (10_340, 10_339),
                (10_350, 10_349),
                (10_360, 10_359),
                (10_356, 10_355),
                (11_618, 11_617),
                (10_362, 10_361),
                (10_358, 10_357),
            ]
        );
    }

    #[test]
    fn original_index_rail_preserves_faction_geometry_and_selected_mode() {
        let alliance = encyclopedia_rail_controls(crate::cockpit::CockpitFaction::Alliance);
        let empire = encyclopedia_rail_controls(crate::cockpit::CockpitFaction::Empire);

        assert_eq!(
            alliance.map(|control| (
                control.command_id,
                control.x,
                control.y,
                control.width,
                control.height,
                control.normal_resource,
                control.pressed_resource,
                control.selected_in_index,
            )),
            [
                (0xfb, 423, 25, 32, 31, 10_370, 10_371, false),
                (0x67, 423, 93, 32, 31, 10_374, 10_375, false),
                (0x68, 423, 147, 32, 31, 10_372, 10_373, true),
            ]
        );
        assert_eq!(
            empire.map(|control| (
                control.command_id,
                control.x,
                control.y,
                control.width,
                control.height,
                control.normal_resource,
                control.pressed_resource,
                control.selected_in_index,
            )),
            [
                (0xfb, 426, 21, 44, 41, 10_376, 10_377, false),
                (0x67, 426, 89, 44, 41, 10_380, 10_381, false),
                (0x68, 426, 143, 44, 41, 10_378, 10_379, true),
            ]
        );
    }

    fn original_catalog_fixture() -> OriginalEncyclopediaCatalog {
        OriginalEncyclopediaCatalog::new(
            "Galactic Encyclopedia".into(),
            "Topic".into(),
            [
                "All Databases".into(),
                "System Database".into(),
                "Ship Database".into(),
                "Facilities Database".into(),
                "Missions Database".into(),
                "Troop Database".into(),
                "Personnel Database".into(),
            ],
            vec![
                OriginalEncyclopediaEntry {
                    object_id: 0x9000_0001,
                    name: "Allyuen".into(),
                },
                OriginalEncyclopediaEntry {
                    object_id: 0x1400_0001,
                    name: "Alliance Dreadnaught".into(),
                },
                OriginalEncyclopediaEntry {
                    object_id: 0x1000_0001,
                    name: "Alliance Army Regiment".into(),
                },
                OriginalEncyclopediaEntry {
                    object_id: 0x3000_0001,
                    name: "Ackbar".into(),
                },
            ],
        )
    }

    #[test]
    fn original_catalog_uses_source_labels_and_family_filters() {
        let catalog = original_catalog_fixture();

        assert_eq!(catalog.len(), 4);
        assert!(!catalog.is_empty());
        assert_eq!(catalog.category_label(0x6f), Some("All Databases"));
        assert_eq!(catalog.category_label(0x75), Some("Personnel Database"));
        assert_eq!(catalog.entries_for(0x6f).len(), 4);
        assert_eq!(catalog.entries_for(0x70)[0].name, "Allyuen");
        assert_eq!(catalog.entries_for(0x71)[0].name, "Alliance Dreadnaught");
        assert_eq!(catalog.entries_for(0x74)[0].name, "Alliance Army Regiment");
        assert_eq!(catalog.entries_for(0x75)[0].name, "Ackbar");
    }

    #[test]
    fn original_catalog_selection_preserves_identity_or_clears() {
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_selected_object_id: Some(0x1400_0001),
            ..EncyclopediaState::default()
        };

        reconcile_original_index_state(&mut state, &catalog, false);
        assert_eq!(state.original_selected_object_id, Some(0x1400_0001));

        state.original_category_command = 0x71;
        reconcile_original_index_state(&mut state, &catalog, true);
        assert_eq!(state.original_selected_object_id, Some(0x1400_0001));

        // FUN_0045f100 rebuilds the list with nothing selected: a category
        // without the object selects none (A0 2026-10-29).
        state.original_category_command = 0x70;
        reconcile_original_index_state(&mut state, &catalog, true);
        assert_eq!(state.original_selected_object_id, None);
        assert_eq!(state.original_scroll, 0);
    }

    #[test]
    fn opening_starts_at_all_databases_with_nothing_selected() {
        // Each opening builds a new window (FUN_0045d400), whose list starts
        // at FUN_0045f100(0x6f, 1).
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_category_command: 0x71,
            original_selected_object_id: Some(0x1400_0001),
            original_scroll: 2,
            ..EncyclopediaState::default()
        };
        open_original_encyclopedia_at(&mut state, &catalog, None);
        assert!(state.open && !state.original_topic_mode);
        assert_eq!(state.original_category_command, 0x6f);
        assert_eq!(state.original_selected_object_id, None);
        assert_eq!(state.original_scroll, 0);
    }

    #[test]
    fn the_index_lists_only_the_games_systems_once_they_are_set() {
        // FUN_00422620 adds the systems of the viewer's side
        // (FUN_004f31b0); the other families stay.
        let entry = |object_id, name: &str| OriginalEncyclopediaEntry {
            object_id,
            name: name.into(),
        };
        let mut catalog = OriginalEncyclopediaCatalog::new(
            "Galactic Encyclopedia".into(),
            "Topic".into(),
            std::array::from_fn(|index| format!("Category {index}")),
            vec![
                entry(0x9000_0001, "Abregado"),
                entry(0x9000_0002, "Coruscant"),
                entry(0x1400_0001, "Star Destroyer"),
            ],
        );
        let ids = |catalog: &OriginalEncyclopediaCatalog, command| -> Vec<u32> {
            catalog.entries_for(command).iter().map(|entry| entry.object_id).collect()
        };
        assert_eq!(ids(&catalog, 0x70), [0x9000_0001, 0x9000_0002]);
        catalog.set_galaxy_systems([0x9000_0002]);
        assert_eq!(ids(&catalog, 0x70), [0x9000_0002]);
        assert_eq!(ids(&catalog, 0x6f), [0x9000_0002, 0x1400_0001]);
        assert_eq!(ids(&catalog, 0x71), [0x1400_0001]);
    }

    #[test]
    fn a_topic_steps_through_its_category_and_returns_to_the_index() {
        // FUN_0045fd20: previous / next exist when the list item has a
        // neighbour; FUN_0045fe60 and commands 0x83 / 0x84 step to it;
        // 0x68 returns to the index (FUN_0045f480). All Databases sorts
        // Ackbar, Alliance Army Regiment, Alliance Dreadnaught, Allyuen.
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_selected_object_id: Some(0x3000_0001),
            original_topic_mode: true,
            ..EncyclopediaState::default()
        };
        assert_eq!(
            original_topic_neighbours(&state, &catalog),
            (None, Some(0x1000_0001))
        );
        apply_original_topic_command(&mut state, &catalog, 0x83);
        assert_eq!(state.original_selected_object_id, Some(0x3000_0001));
        apply_original_topic_command(&mut state, &catalog, 0x84);
        assert_eq!(state.original_selected_object_id, Some(0x1000_0001));
        apply_original_topic_command(&mut state, &catalog, 0x84);
        apply_original_topic_command(&mut state, &catalog, 0x84);
        assert_eq!(state.original_selected_object_id, Some(0x9000_0001));
        assert_eq!(
            original_topic_neighbours(&state, &catalog),
            (Some(0x1400_0001), None)
        );
        apply_original_topic_command(&mut state, &catalog, 0x84);
        assert_eq!(state.original_selected_object_id, Some(0x9000_0001));
        // Within a category, only its objects.
        state.original_category_command = 0x70;
        assert_eq!(original_topic_neighbours(&state, &catalog), (None, None));
        assert!(state.original_topic_mode);
        apply_original_topic_command(&mut state, &catalog, 0x68);
        assert!(!state.original_topic_mode);
        // An unknown selection has no neighbours.
        state.original_selected_object_id = Some(0x5000_0001);
        assert_eq!(original_topic_neighbours(&state, &catalog), (None, None));
    }

    fn run_topic(
        state: &mut EncyclopediaState,
        catalog: &OriginalEncyclopediaCatalog,
        scale: f32,
        events: Vec<egui::Event>,
    ) -> (Option<u16>, Vec<egui::epaint::ClippedShape>, egui::Context) {
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let topic = OriginalEncyclopediaTopic {
            title: "Ackbar".into(),
            body: Some("A Mon Calamari admiral.".into()),
            edata: None,
        };
        let mut activated = None;
        let screen = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
        ));
        // egui draws an Area from its second frame (the first sizes it).
        let mut warm_up = EncyclopediaState {
            original_category_command: state.original_category_command,
            original_selected_object_id: state.original_selected_object_id,
            original_topic_mode: state.original_topic_mode,
            ..EncyclopediaState::default()
        };
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                ..Default::default()
            },
            |ctx| {
                let _ = draw_encyclopedia_topic(
                    ctx,
                    &mut cache,
                    crate::cockpit::CockpitFaction::Alliance,
                    egui::pos2(10.0, 20.0),
                    scale,
                    &mut warm_up,
                    catalog,
                    &topic,
                );
            },
        );
        let output = ctx.run(
            egui::RawInput {
                screen_rect: screen,
                events,
                ..Default::default()
            },
            |ctx| {
                activated = draw_encyclopedia_topic(
                    ctx,
                    &mut cache,
                    crate::cockpit::CockpitFaction::Alliance,
                    egui::pos2(10.0, 20.0),
                    scale,
                    state,
                    catalog,
                    &topic,
                );
            },
        );
        (activated, output.shapes, ctx)
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn the_topics_keys_step_through_the_category() {
        // FUN_0045fe60: in topic mode Left and Right are 0x83 and 0x84.
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_selected_object_id: Some(0x3000_0001),
            original_topic_mode: true,
            ..EncyclopediaState::default()
        };
        let (activated, _, _) =
            run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowLeft)]);
        assert_eq!(activated, None);
        let (activated, _, _) =
            run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowRight)]);
        assert_eq!(activated, Some(0x84));
        assert_eq!(state.original_selected_object_id, Some(0x1000_0001));
        let (activated, _, _) =
            run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowLeft)]);
        assert_eq!(activated, Some(0x83));
        assert_eq!(state.original_selected_object_id, Some(0x3000_0001));
        // No window at a zero scale.
        let (activated, shapes, _) =
            run_topic(&mut state, &catalog, 0.0, vec![key(egui::Key::ArrowRight)]);
        assert_eq!(activated, None);
        assert!(shapes.is_empty());
    }

    /// Twelve systems, E00..E11.
    fn twelve_entry_catalog() -> OriginalEncyclopediaCatalog {
        OriginalEncyclopediaCatalog::new(
            "Galactic Encyclopedia".into(),
            "Topic".into(),
            std::array::from_fn(|index| format!("Category {index}")),
            (0..12)
                .map(|index| OriginalEncyclopediaEntry {
                    object_id: 0x9000_0000 + index,
                    name: format!("E{index:02}"),
                })
                .collect(),
        )
    }

    /// Run the index at scale 1 from (10, 20): two idle frames, then one
    /// per entry of `frames`, the pointer at window point `pointer`.
    /// Returns the last frame's texts (name, top-left, size) and the
    /// bitmaps painted in it.
    fn run_index(
        state: &mut EncyclopediaState,
        catalog: &OriginalEncyclopediaCatalog,
        pointer: (f32, f32),
        frames: Vec<Vec<egui::Event>>,
    ) -> (Vec<(String, egui::Pos2, egui::Vec2)>, Vec<(u32, egui::Pos2)>) {
        run_index_at(state, catalog, pointer, frames, 1.0)
    }

    /// [`run_index`] at `scale`.
    fn run_index_at(
        state: &mut EncyclopediaState,
        catalog: &OriginalEncyclopediaCatalog,
        pointer: (f32, f32),
        frames: Vec<Vec<egui::Event>>,
        scale: f32,
    ) -> (Vec<(String, egui::Pos2, egui::Vec2)>, Vec<(u32, egui::Pos2)>) {
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let pos = egui::pos2(10.0 + pointer.0 * scale, 20.0 + pointer.1 * scale);
        let mut texts = Vec::new();
        for extra in [vec![], vec![]].into_iter().chain(frames) {
            crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend(extra);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_encyclopedia_index_catalog(
                        ctx,
                        &mut cache,
                        crate::cockpit::CockpitFaction::Alliance,
                        egui::pos2(10.0, 20.0),
                        scale,
                        state,
                        catalog,
                    );
                },
            );
            texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => {
                        Some((text.galley.text().to_owned(), text.pos, text.galley.size()))
                    }
                    _ => None,
                })
                .collect();
        }
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
        (texts, painted)
    }

    fn button(pos: (f32, f32), pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(10.0 + pos.0, 20.0 + pos.1),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn at_scale_two_the_down_arrow_still_steps_ten_and_a_wheel_notch_a_row() {
        // The bar reads the pointer in window pixels at any scale. port: the
        // original has no wheel; a notch moves a 20-pixel row.
        let catalog = twelve_entry_catalog();
        let arrow = (380.0, 292.0);
        let at = |(x, y): (f32, f32)| egui::pos2(10.0 + x * 2.0, 20.0 + y * 2.0);
        let press = |pressed| egui::Event::PointerButton {
            pos: at(arrow),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut state = EncyclopediaState::default();
        let _ = run_index_at(&mut state, &catalog, arrow, vec![vec![press(true)], vec![press(false)]], 2.0);
        assert_eq!(state.original_scroll, 10);

        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            modifiers: egui::Modifiers::NONE,
        };
        let mut state = EncyclopediaState::default();
        let _ = run_index(&mut state, &catalog, (200.0, 200.0), vec![vec![wheel.clone()]]);
        assert_eq!(state.original_scroll, 20);
        // Outside the list the wheel does nothing.
        let mut state = EncyclopediaState::default();
        let _ = run_index(&mut state, &catalog, (200.0, 20.0), vec![vec![wheel]]);
        assert_eq!(state.original_scroll, 0);
    }

    #[test]
    fn the_index_list_scrolls_by_pixels_and_clips_its_rows() {
        // The list scrolls by pixels (FUN_0060a490): scrolled 30, E01 sits
        // at 137 + 20 - 30 = 127, centred on 137; E00 is gone; E09, at
        // 287, still shows its top; E10 does not.
        let catalog = twelve_entry_catalog();
        let mut state = EncyclopediaState {
            original_scroll: 30,
            ..EncyclopediaState::default()
        };
        let (texts, _) = run_index(&mut state, &catalog, (0.0, 0.0), vec![]);
        let centre = |name: &str| {
            texts
                .iter()
                .find(|(text, ..)| text == name)
                .map(|(_, pos, size)| pos.y + size.y / 2.0)
        };
        assert_eq!(centre("E00"), None);
        assert_eq!(centre("E01"), Some(20.0 + 137.0));
        assert_eq!(centre("E09"), Some(20.0 + 297.0));
        assert_eq!(centre("E10"), None);
        assert_eq!(state.original_scroll, 30);
    }

    #[test]
    fn the_index_bar_sits_right_of_the_list_and_its_down_arrow_steps_ten() {
        // FUN_0045ddc0's bar (RCDATA 10653) at the list's right, x 374, from
        // y 137 to 297; a click on the down arrow scrolls by the list's 10.
        let catalog = twelve_entry_catalog();
        let mut state = EncyclopediaState::default();
        let arrow = (380.0, 292.0);
        let (_, painted) = run_index(
            &mut state,
            &catalog,
            arrow,
            vec![vec![button(arrow, true)], vec![button(arrow, false)]],
        );
        assert_eq!(state.original_scroll, 10);
        let at = |x: f32, y: f32| egui::pos2(10.0 + x, 20.0 + y);
        assert!(painted.contains(&(10_685, at(374.0, 137.0))), "{painted:?}");
        assert!(painted.contains(&(10_687, at(374.0, 288.0))), "{painted:?}");
        assert!(painted.contains(&(10_656, at(374.0, 146.0))), "{painted:?}");
        // Held on the arrow, it draws pressed; off it, not.
        let mut state = EncyclopediaState::default();
        let (_, painted) = run_index(&mut state, &catalog, arrow, vec![vec![button(arrow, true)], vec![]]);
        assert!(painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
        let mut state = EncyclopediaState::default();
        let (_, painted) =
            run_index(&mut state, &catalog, (100.0, 100.0), vec![vec![button(arrow, true)], vec![]]);
        assert!(!painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
        // A row clicked under the bar's column is not selected.
        let mut state = EncyclopediaState::default();
        let row = (378.0, 150.0);
        let _ = run_index(&mut state, &catalog, row, vec![vec![button(row, true)], vec![button(row, false)]]);
        assert_eq!(state.original_selected_object_id, None);
    }

    #[test]
    fn index_rows_are_grey_but_the_selected_white_centred_in_twenty_pixel_rows() {
        // FUN_0045ddc0's list: text at x 5 of the 350-wide cell, centred on
        // its 20-pixel row (+0xe4 = 0x24), font 10, grey 0x787878 or white
        // when selected (FUN_00607ea0); the title and category label in
        // font 5. At scale 2 from (10, 20).
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_selected_object_id: Some(0x1000_0001),
            ..EncyclopediaState::default()
        };
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_encyclopedia_index_catalog(
                        ctx,
                        &mut cache,
                        crate::cockpit::CockpitFaction::Alliance,
                        egui::pos2(10.0, 20.0),
                        2.0,
                        &mut state,
                        &catalog,
                    );
                },
            );
            shapes = output.shapes;
        }
        // The selector also names the selected object; the rows start at
        // the cell's x 5.
        let row_x = 10.0 + 41.0 * 2.0;
        let text = |wanted: &str| {
            shapes
                .iter()
                .filter(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => {
                        !wanted.starts_with("Alliance") || text.pos.x == row_x
                    }
                    _ => false,
                })
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == wanted => Some((
                        text.pos,
                        text.galley.size(),
                        text.galley.job.sections[0].format.font_id.size,
                        text.galley.job.sections[0].format.color,
                    )),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{wanted} not drawn"))
        };
        let (pos, size, font, color) = text("Ackbar");
        assert_eq!(pos.x, row_x);
        assert!((pos.y + size.y / 2.0 - (20.0 + 147.0 * 2.0)).abs() < 0.01, "{pos:?}");
        assert_eq!(font, crate::theme::original_font(10, 2.0).size);
        assert_eq!(color, ORIGINAL_INDEX_ROW_COLOR);
        let (pos, size, _, color) = text("Alliance Army Regiment");
        assert!((pos.y + size.y / 2.0 - (20.0 + 167.0 * 2.0)).abs() < 0.01, "{pos:?}");
        assert_eq!(color, Color32::WHITE);
        assert_eq!(text("Galactic Encyclopedia").2, crate::theme::original_font(5, 2.0).size);
        assert_eq!(text("All Databases").2, crate::theme::original_font(5, 2.0).size);
        assert_eq!(text("Topic").2, crate::theme::original_font(4, 2.0).size);
    }

    #[test]
    fn the_topic_lays_out_its_title_and_body_at_the_original_points() {
        // FUN_0045ddc0: the 470 x 330 client; the title in (36, 14, 350);
        // the body in (17, 231, 395, 80); here at scale 2 from (10, 20).
        let catalog = original_catalog_fixture();
        let mut state = EncyclopediaState {
            original_selected_object_id: Some(0x3000_0001),
            original_topic_mode: true,
            ..EncyclopediaState::default()
        };
        let (_, shapes, ctx) = run_topic(&mut state, &catalog, 2.0, Vec::new());
        let area = ctx
            .memory(|memory| memory.area_rect(egui::Id::new("original-encyclopedia-topic")))
            .expect("the topic window");
        assert_eq!(area.min, egui::pos2(10.0, 20.0));
        assert_eq!(area.size(), egui::vec2(470.0 * 2.0, 330.0 * 2.0));
        let texts: Vec<(String, egui::Pos2, egui::Rect)> = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    Some((text.galley.text().to_owned(), text.pos, clipped.clip_rect))
                }
                _ => None,
            })
            .collect();
        // The title in font 5 (the label +0x120); the body at a 13-pixel
        // em, lines 16 apart (A0 2026-10-29).
        let font_size = |wanted: &str| {
            shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with(wanted) => {
                    Some(text.galley.job.sections[0].format.font_id.size)
                }
                _ => None,
            })
        };
        assert_eq!(font_size("Ackbar"), Some(crate::theme::original_font(5, 2.0).size));
        assert_eq!(font_size("A Mon Calamari"), Some(26.0));
        // The body's lines 16 apart (A0 2026-10-29), white.
        let body = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with("A Mon Calamari") => {
                    Some(text.galley.clone())
                }
                _ => None,
            })
            .expect("the body");
        assert_eq!(body.job.sections[0].format.line_height, Some(32.0));
        assert_eq!(body.job.sections[0].format.color, Color32::WHITE);
        let (_, title_pos, title_clip) = texts
            .iter()
            .find(|(text, _, _)| text == "Ackbar")
            .expect("the title");
        assert_eq!(title_pos.y, 20.0 + 14.0 * 2.0);
        assert_eq!(
            *title_clip,
            egui::Rect::from_min_size(egui::pos2(10.0 + 36.0 * 2.0, 48.0), egui::vec2(700.0, 34.0))
        );
        let (_, body_pos, body_clip) = texts
            .iter()
            .find(|(text, _, _)| text.starts_with("A Mon Calamari"))
            .expect("the body");
        assert_eq!(*body_pos, egui::pos2(10.0 + 17.0 * 2.0, 20.0 + 231.0 * 2.0));
        assert_eq!(body_clip.size(), egui::vec2(395.0 * 2.0, 80.0 * 2.0));
    }

    #[test]
    fn a_menu_object_opens_the_encyclopedia_at_its_topic() {
        // FUN_0045d400 resolves the initial object: its topic when the
        // catalog lists it, else the index.
        use crate::object_menu::MenuObject;
        let catalog = original_catalog_fixture();
        let mut world = GameWorld::default();
        let ackbar = world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3000_0001),
            ..Default::default()
        });
        let unknown = world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3000_0099),
            ..Default::default()
        });
        assert_eq!(
            original_encyclopedia_object(&world, MenuObject::Character(ackbar)),
            Some(0x3000_0001)
        );
        assert_eq!(
            original_encyclopedia_object(&world, MenuObject::Fleet(Default::default())),
            None
        );

        let mut state = EncyclopediaState {
            original_category_command: 0x70,
            ..EncyclopediaState::default()
        };
        open_original_encyclopedia_at(&mut state, &catalog, Some(0x3000_0001));
        assert!(state.open && state.original_topic_mode);
        assert_eq!(state.original_category_command, 0x6f);
        assert_eq!(state.original_selected_object_id, Some(0x3000_0001));

        let mut state = EncyclopediaState::default();
        let object = original_encyclopedia_object(&world, MenuObject::Character(unknown));
        open_original_encyclopedia_at(&mut state, &catalog, object);
        assert!(state.open && !state.original_topic_mode);
        assert_eq!(state.original_selected_object_id, None);
    }

    #[test]
    fn a_topics_artwork_name_gives_its_edata_number() {
        assert_eq!(
            OriginalEncyclopediaTopic::edata_number("EDATA.014"),
            Some(14)
        );
        assert_eq!(
            OriginalEncyclopediaTopic::edata_number("edata.187"),
            Some(187)
        );
        assert_eq!(OriginalEncyclopediaTopic::edata_number("EDATA.x"), None);
        assert_eq!(OriginalEncyclopediaTopic::edata_number("BMP.014"), None);
    }

    #[test]
    fn original_category_ranges_preserve_every_exclusive_boundary() {
        assert!(original_category_contains(0x6f, 0));
        for (command, start, end) in [
            (0x70, 0x90, 0x98),
            (0x71, 0x14, 0x20),
            (0x72, 0x20, 0x30),
            (0x73, 0x40, 0x80),
            (0x74, 0x10, 0x14),
            (0x75, 0x30, 0x40),
        ] {
            assert!(!original_category_contains(command, start - 1));
            assert!(original_category_contains(command, start));
            assert!(original_category_contains(command, end - 1));
            assert!(!original_category_contains(command, end));
        }
        assert!(!original_category_contains(0x76, 0x30));
    }

    #[test]
    fn original_catalog_reveal_and_clamp_keep_selection_visible() {
        let entries = (0..12)
            .map(|index| OriginalEncyclopediaEntry {
                object_id: 0x9000_0000 | index,
                name: format!("System {index:02}"),
            })
            .collect();
        let catalog = OriginalEncyclopediaCatalog::new(
            "Galactic Encyclopedia".into(),
            "Topic".into(),
            std::array::from_fn(|index| format!("Category {index}")),
            entries,
        );
        let mut state = EncyclopediaState {
            original_category_command: 0x70,
            original_selected_object_id: Some(0x9000_000b),
            original_scroll: 0,
            ..EncyclopediaState::default()
        };

        // Twelve entries of 20 pixels in the 160-pixel list (FUN_00607ea0),
        // scrolled by pixels: the last one shows from pixel 80, and the
        // range ends at 240 + 5 - 160 (FUN_00609ae0).
        reconcile_original_index_state(&mut state, &catalog, true);
        assert_eq!(state.original_scroll, 80);

        state.original_selected_object_id = Some(0x9000_0000);
        reconcile_original_index_state(&mut state, &catalog, true);
        assert_eq!(state.original_scroll, 0);

        state.original_scroll = usize::MAX;
        reconcile_original_index_state(&mut state, &catalog, false);
        assert_eq!(state.original_scroll, 85);

        // FUN_0045f100 rebuilds the list with nothing selected.
        state.original_category_command = 0xffff;
        state.original_selected_object_id = Some(0xffff_ffff);
        reconcile_original_index_state(&mut state, &catalog, false);
        assert_eq!(state.original_category_command, 0x6f);
        assert_eq!(state.original_selected_object_id, None);
        assert_eq!(state.original_scroll, 0);
    }

    #[test]
    fn eight_twenty_pixel_rows_fill_the_native_list() {
        // FUN_00607ea0(.., 0x24, 0x89, 0x15e, 0xa0, .., 0x15e, 0x14, ..):
        // 350 by 20 cells in the 160-pixel list (A0 2026-10-29: eight rows,
        // 20 apart).
        let parent = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(470.0, 330.0));
        let list = original_index_list_rect(parent, 1.0);

        for row in 0..ORIGINAL_INDEX_VISIBLE_ROWS {
            #[expect(clippy::cast_precision_loss, reason = "Eight rows.")]
            let rect = original_index_row_rect(parent, 1.0, 137.0 + row as f32 * 20.0);
            assert!(rect.min.y >= list.min.y);
            assert!(rect.max.y <= list.max.y);
            assert_eq!(rect.height(), 20.0);
        }
        assert_eq!(ORIGINAL_INDEX_VISIBLE_ROWS, 8);
        assert_eq!(original_index_row_rect(parent, 1.0, 277.0).max.y, list.max.y);
        // A row scrolled half out is clipped to the list.
        assert_eq!(original_index_row_rect(parent, 1.0, 127.0).height(), 10.0);
    }

    #[test]
    fn the_window_sits_centred_in_each_sides_galaxy_view() {
        // A0 2026-10-29 (Empire): (125, 52); captures 281117 and 281123
        // (Alliance): (62, 50). FUN_00606980's truncating halves of the
        // galaxy view (FUN_00421c70) less 470 by 330.
        use crate::cockpit::{CockpitFaction, CockpitLayout, CockpitViewport};
        for (faction, origin) in [
            (CockpitFaction::Alliance, (62.0, 50.0)),
            (CockpitFaction::Empire, (125.0, 52.0)),
        ] {
            let (x, y, width, height) = crate::cockpit::galaxy_aperture(faction);
            let layout = CockpitLayout {
                canvas: CockpitViewport {
                    x: 10.0,
                    y: 20.0,
                    width: 1280.0,
                    height: 960.0,
                },
                galaxy: CockpitViewport {
                    x: 10.0 + x * 2.0,
                    y: 20.0 + y * 2.0,
                    width: width * 2.0,
                    height: height * 2.0,
                },
                scale: 2.0,
            };
            assert_eq!(
                encyclopedia_origin(layout),
                egui::pos2(10.0 + origin.0 * 2.0, 20.0 + origin.1 * 2.0)
            );
        }
    }
}
