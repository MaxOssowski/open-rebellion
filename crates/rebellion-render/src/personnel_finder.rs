//! The Personnel Finder (window type `0x17`, `FUN_0042a4d0`;
//! `ghidra/notes/personnel-finder.md`).
//!
//! A 470 by 330 window the cockpit's Personnel Finder control or F4
//! opens. Two side tabs (Alliance, Imperial) choose the faction; two view
//! tabs choose between a Characters list and a Special Forces list.
//!
//! Characters view: rows are "Name - Location (state) (rank)"
//! (`FUN_00465bb0`). Special Forces view: rows per system with four
//! count cells by class (`FUN_00465540`). Typing a name picks the best
//! match (`FUN_00609650`); Display, Enter or a double click opens the
//! appropriate window (`FUN_00429440`).

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{CharacterKey, FleetKey, SpecialForceKey, SystemKey};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{fleet_label, logical_rect, rect_contains};

pub const PERSONNEL_FINDER_WIDTH: f32 = 470.0;
pub const PERSONNEL_FINDER_HEIGHT: f32 = 330.0;

// STRATEGY bitmaps by side (FUN_004637a0).
const BACKGROUND: [u32; 2] = [10335, 10336];
const RAIL: [u32; 2] = [10586, 10590];
const RAIL_AT: (f32, f32) = (412.0, 0.0);
const PANEL_AT: (f32, f32) = (12.0, 13.0);

/// Panel bitmaps indexed by `[view][side]`. `FUN_004637a0` lays 10538,
/// 10539, 10536 and 10537 on the window's panels 2, 3, 4 and 5
/// (`0x463b78..0x463bdf`); `FUN_00464e10` shows panel 2 or 3 (Alliance or
/// Empire) for view 2, SpecForces, and 4 or 5 for view 3, Characters. 10538
/// carries the four SpecForces column icons.
const PANELS: [[u32; 2]; 2] = [
    [10536, 10537], // Characters view: Alliance, Imperial
    [10538, 10539], // SpecForces view: Alliance, Imperial
];

/// The side tab (FUN_004637a0's control 100, TEXTSTRA 0x1893/0x1894).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersonnelTab {
    #[default]
    Alliance,
    Imperial,
}

impl PersonnelTab {
    const ALL: [Self; 2] = [Self::Alliance, Self::Imperial];

    fn lists(self, is_alliance: bool) -> bool {
        match self {
            Self::Alliance => is_alliance,
            Self::Imperial => !is_alliance,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Alliance => "Alliance Personnel",
            Self::Imperial => "Imperial Personnel",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Alliance => 0,
            Self::Imperial => 1,
        }
    }
}

/// The view tab (FUN_004637a0's control 0x6e): value 3 is Characters
/// (TEXTSTRA `0x1899`), value 2 SpecForces (`0x1898`); `FUN_00464e10` lists
/// characters for 3 and special forces for 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersonnelView {
    #[default]
    Characters,
    SpecForces,
}

impl PersonnelView {
    const ALL: [Self; 2] = [Self::SpecForces, Self::Characters];

    const fn index(self) -> usize {
        match self {
            Self::Characters => 0,
            Self::SpecForces => 1,
        }
    }

    /// The view button's offset in its group (`FUN_004637a0`): Characters at
    /// the top, SpecForces `0x36` lower.
    const fn button_offset(self) -> f32 {
        match self {
            Self::Characters => 0.0,
            Self::SpecForces => 54.0,
        }
    }

    /// The tab label y (`FUN_00464e10`): `0x77` (119) for view 2,
    /// SpecForces, else `0x73` (115).
    const fn label_y(self) -> f32 {
        match self {
            Self::Characters => 115.0,
            Self::SpecForces => 119.0,
        }
    }
}

/// The side's Close and Display controls (FUN_004637a0).
struct SideControls {
    close: Control,
    display: Control,
    /// View tab group origin and individual button size.
    view_origin: (f32, f32),
    view_button_size: (f32, f32),
}

#[derive(Debug, Clone, Copy)]
struct Control {
    rect: (f32, f32, f32, f32),
    art: (u32, u32),
}

const CONTROLS: [SideControls; 2] = [
    // Alliance (side 1): FUN_004637a0.
    SideControls {
        close: Control {
            rect: (423.0, 25.0, 32.0, 31.0),
            art: (10514, 10515),
        },
        display: Control {
            rect: (423.0, 93.0, 32.0, 31.0),
            art: (10518, 10519),
        },
        view_origin: (423.0, 147.0),
        view_button_size: (32.0, 31.0),
    },
    // Empire (side 2).
    SideControls {
        close: Control {
            rect: (426.0, 21.0, 44.0, 41.0),
            art: (10516, 10517),
        },
        display: Control {
            rect: (426.0, 89.0, 42.0, 41.0),
            art: (10520, 10521),
        },
        view_origin: (426.0, 143.0),
        view_button_size: (42.0, 41.0),
    },
];

/// View tab art: `[side][view]` (FUN_004637a0). Characters (value 3, at the
/// group's top): Alliance 10542/10543, Empire 10544/10545, a face.
/// SpecForces (value 2, `0x36` lower): Alliance 10546/10547, Empire
/// 10548/10549, two soldiers.
const VIEW_TAB_ART: [[(u32, u32); 2]; 2] = [
    // Alliance: [Characters, SpecForces]
    [(10542, 10543), (10546, 10547)],
    // Empire: [Characters, SpecForces]
    [(10544, 10545), (10548, 10549)],
];

/// Side tabs (group 100) at (36, 72): same as troop/fleet finder.
const SIDE_TABS: [Control; 2] = [
    Control {
        rect: (36.0, 72.0, 49.0, 41.0),
        art: (10502, 10503),
    },
    Control {
        rect: (88.0, 72.0, 49.0, 41.0),
        art: (10505, 10506),
    },
];

const TITLE_AT: (f32, f32) = (36.0, 14.0);
/// The list's text x (`+0xe8 = 5`) and unchosen colour (`0x787878`).
const ROW_TEXT_X: f32 = 5.0;
const ROW_COLOR: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
const NAME_LABEL_AT: (f32, f32) = (36.0, 48.0);
const NAME_BOX: (f32, f32, f32, f32) = (143.0, 45.0, 250.0, 18.0);
const LIST: (f32, f32, f32, f32) = (37.0, 143.0, 349.0, 161.0);
const ROW_HEIGHT: f32 = 20.0;

/// Four Special Forces DatIds per side (FUN_00465540).
const ALLIANCE_SF_CLASSES: [u32; 4] = [0x3c00_0004, 0x3c00_0002, 0x3c00_0001, 0x3c00_0003];
const EMPIRE_SF_CLASSES: [u32; 4] = [0x3c00_0007, 0x3c00_0006, 0x3c00_0008, 0x3c00_0005];

fn sf_classes(tab: PersonnelTab) -> &'static [u32; 4] {
    match tab {
        PersonnelTab::Alliance => &ALLIANCE_SF_CLASSES,
        PersonnelTab::Imperial => &EMPIRE_SF_CLASSES,
    }
}

/// One count cell in a SpecForces row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SfCount {
    pub count: u32,
    /// port: always true (no usable bit in the port).
    pub all_usable: bool,
}

/// One list row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonnelFinderRow {
    /// A character: "Name - Location (state)" (FUN_00465bb0).
    Character {
        key: CharacterKey,
        name: String,
        location: String,
        state: String,
        system: Option<SystemKey>,
    },
    /// A system (or fleet) with SpecForces counts (FUN_00465540).
    SpecForces {
        name: String,
        system: SystemKey,
        fleet: Option<FleetKey>,
        counts: [SfCount; 4],
    },
}

impl PersonnelFinderRow {
    fn display_name(&self) -> &str {
        match self {
            Self::Character { name, .. } => name,
            Self::SpecForces { name, .. } => name,
        }
    }
}

/// A typed name chooses the first row it best begins, or the first row
/// (FUN_00609650, [`crate::list_prefix::best_prefix_row`]).
#[must_use]
pub fn prefix_choice(rows: &[PersonnelFinderRow], text: &str) -> Option<usize> {
    crate::list_prefix::best_prefix_row(rows.iter().map(|row| row.display_name()), text)
}

/// Count special forces of each class among the given keys.
fn count_sf(world: &GameWorld, keys: &[SpecialForceKey], tab: PersonnelTab) -> [SfCount; 4] {
    let classes = sf_classes(tab);
    let mut counts = [SfCount {
        count: 0,
        all_usable: true,
    }; 4];
    for &key in keys {
        let Some(sf) = world.special_forces.get(key) else {
            continue;
        };
        if !tab.lists(sf.is_alliance) {
            continue;
        }
        let raw = sf.class_dat_id.raw();
        for (i, &class_raw) in classes.iter().enumerate() {
            if raw == class_raw {
                counts[i].count += 1;
                break;
            }
        }
    }
    counts
}

/// A character's state word (`FUN_00465bb0`): Captured (`0x8746`) when held
/// (`+0xac` bit 0), else On Mission (`0x8745`), else Injured (`0x8747`,
/// `+0x94`), else Enroute (`0x8744`); the first that applies. port: a
/// character off the map, in no fleet and in no abode is the one travelling.
fn character_state(character: &rebellion_core::world::Character) -> Option<&'static str> {
    if character.is_captive {
        Some("Captured")
    } else if character.on_mission {
        Some("On Mission")
    } else if character.injury != 0 {
        Some("Injured")
    } else if character.current_system.is_none()
        && character.current_fleet.is_none()
        && character.current_abode.is_none()
    {
        Some("Enroute")
    } else {
        None
    }
}

/// A character's command word (`FUN_00465bb0`, `+0x96`): 1 Commander
/// (`0x8802`), 2 Admiral (`0x8801`), 3 General (`0x8800`).
fn command_word(command: rebellion_core::character_command::CommandKind) -> Option<&'static str> {
    use rebellion_core::character_command::CommandKind;
    match command {
        CommandKind::None => None,
        CommandKind::Commander => Some("Commander"),
        CommandKind::Admiral => Some("Admiral"),
        CommandKind::General => Some("General"),
    }
}

/// The window a character's Display opens (`FUN_00429440`, family `0x30`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTarget {
    /// Aboard: its fleet's Fleet window (kind 4).
    Fleet(FleetKey),
    /// At a system on a mission that is not hidden (`FUN_00520b70` is 0):
    /// the Missions window (kind 11).
    Missions,
    /// At a system otherwise: the Defenses window (kind 10).
    Defenses,
}

/// Which window `character`'s Display opens (`FUN_00429440`): its mission
/// key `+0x68` names a mission (`FUN_004f3000`); a hidden one opens the
/// Defenses window, where its member still lists.
#[must_use]
pub fn character_target(character: &rebellion_core::world::Character) -> CharacterTarget {
    match character.current_fleet {
        Some(fleet) => CharacterTarget::Fleet(fleet),
        None if character.on_mission && !character.on_hidden_mission => CharacterTarget::Missions,
        None => CharacterTarget::Defenses,
    }
}

/// A Characters row's text (`FUN_00465bb0`): the name and " - " (`0x1897`),
/// then:
/// - for a destroyed character (`+0x50` bit 3), "Killed" (`0x8798`), or
///   "Retired" (`0x8799`) when its destruction reason (`+0x40`) is
///   `0x14..0x16`, as a Retire order's (`FUN_00534c00`) is;
/// - in an abode (container family `0x98..0x9f`), the abode's name alone;
/// - otherwise its container's name ("Location Unknown", `0x1895`, with
///   none), the state word, then the command word, each in " ( " and " ) "
///   (`DAT_006a8790`, `DAT_006a878c`).
///
/// port: a character aboard names its fleet, not its ship.
fn character_text(world: &GameWorld, character: &rebellion_core::world::Character) -> String {
    if character.is_killed {
        let word = if character.is_retired {
            "Retired"
        } else {
            "Killed"
        };
        return format!("{} - {word}", character.name);
    }
    if let Some(abode) = character
        .current_abode
        .and_then(|abode| world.abodes.get(abode))
    {
        return format!("{} - {}", character.name, abode.name);
    }
    let location = character
        .current_fleet
        .and_then(|fleet| fleet_label(world, fleet))
        .or_else(|| {
            character
                .current_system
                .and_then(|system| world.systems.get(system))
                .map(|system| system.name.clone())
        })
        .unwrap_or_else(|| "Location Unknown".to_owned());
    let mut text = format!("{} - {location}", character.name);
    for word in [character_state(character), command_word(character.command)]
        .into_iter()
        .flatten()
    {
        text.push_str(&format!(" ( {word} ) "));
    }
    text
}

/// Build the Characters view rows for `tab` (FUN_00464e10 view 3,
/// FUN_00465bb0).
#[must_use]
pub fn character_rows(
    world: &GameWorld,
    _fog: &FogState,
    _player: Faction,
    tab: PersonnelTab,
) -> Vec<PersonnelFinderRow> {
    let mut rows = Vec::new();
    for (key, character) in &world.characters {
        if !character.recruited {
            continue;
        }
        if !tab.lists(character.is_alliance) {
            continue;
        }
        // hyp: fog gating for opposing side's characters.
        let location = character
            .current_system
            .and_then(|sys| world.systems.get(sys))
            .map_or_else(String::new, |sys| sys.name.clone());
        let state = character_state(character).unwrap_or_default().to_owned();
        let display = character_text(world, character);
        rows.push(PersonnelFinderRow::Character {
            key,
            name: display,
            location,
            state,
            system: character.current_system,
        });
    }
    rows.sort_by_cached_key(|row| row.display_name().to_ascii_lowercase());
    rows
}

/// Build the SpecForces view rows for `tab` (FUN_00464e10 view 2,
/// FUN_00465540).
#[must_use]
pub fn specforces_rows(
    world: &GameWorld,
    _fog: &FogState,
    _player: Faction,
    tab: PersonnelTab,
) -> Vec<PersonnelFinderRow> {
    let mut rows = Vec::new();

    for (system_key, system) in &world.systems {
        // Direct SF units at this system.
        let ground: Vec<SpecialForceKey> = system
            .special_forces
            .iter()
            .copied()
            .filter(|&key| {
                world
                    .special_forces
                    .get(key)
                    .is_some_and(|sf| tab.lists(sf.is_alliance))
            })
            .collect();
        if !ground.is_empty() {
            let counts = count_sf(world, &ground, tab);
            rows.push(PersonnelFinderRow::SpecForces {
                name: system.name.clone(),
                system: system_key,
                fleet: None,
                counts,
            });
        }

        // FUN_00464e10: then each fleet of the tab's side at the system
        // (FUN_004ffef0) whose ships carry special forces is a row of its
        // own, counted by FUN_00465540 over the fleet.
        for &fleet_key in &system.fleets {
            let Some(fleet) = world.fleets.get(fleet_key) else {
                continue;
            };
            if !tab.lists(fleet.is_alliance) || fleet.special_forces.is_empty() {
                continue;
            }
            rows.push(PersonnelFinderRow::SpecForces {
                name: crate::system_window::fleet_label(world, fleet_key).unwrap_or_default(),
                system: system_key,
                fleet: Some(fleet_key),
                counts: count_sf(world, &fleet.special_forces, tab),
            });
        }
    }

    rows.sort_by_cached_key(|row| row.display_name().to_ascii_lowercase());
    rows
}

/// Build rows for the current view.
#[must_use]
pub fn rows(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: PersonnelTab,
    view: PersonnelView,
) -> Vec<PersonnelFinderRow> {
    match view {
        PersonnelView::Characters => character_rows(world, fog, player, tab),
        PersonnelView::SpecForces => specforces_rows(world, fog, player, tab),
    }
}

/// What the Personnel Finder asks of the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonnelFinderAction {
    /// Character row: open the character window (FUN_00429440 family 0x30).
    OpenCharacter {
        character: CharacterKey,
        system: Option<SystemKey>,
    },
    /// SpecForces system row: open the Sector and System windows
    /// (FUN_00429440 family 0x90).
    OpenSystem { system: SystemKey },
    /// SpecForces fleet row: open the Fleet window.
    OpenFleet { system: SystemKey, fleet: FleetKey },
}

#[derive(Debug, Clone)]
struct OpenFinder {
    faction: CockpitFaction,
    tab: PersonnelTab,
    view: PersonnelView,
    name: String,
    chosen: Option<usize>,
    /// The list's first visible pixel (it scrolls by pixels,
    /// `FUN_0060a490`).
    scroll: usize,
    /// A press held on the list's scroll bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    focus_name: bool,
    last_clicked: Option<(usize, f64)>,
}

/// The Personnel Finder, one at most (FUN_0042a4d0).
#[derive(Debug, Clone, Default)]
pub struct PersonnelFinderState {
    window: Option<OpenFinder>,
}

/// The list's height and rows in pixels, its bar's width (RCDATA 10653,
/// `FUN_004637a0`: `FUN_0060a490(.., list, 7, 0x299d, ..)`) and arrow step
/// (`FUN_00607ea0`'s argument 10).
const LIST_PIXELS: usize = 161;
const ROW_PIXELS: usize = 20;
const BAR_WIDTH: f32 = 12.0;
const LIST_STEP: f32 = 10.0;

/// The scroll that shows `row` whole, moving the list as little as
/// possible, within the range of `rows` rows.
fn scrolled_to(scroll: usize, row: usize, rows: usize) -> usize {
    let top = row * ROW_PIXELS;
    let bottom = top + ROW_PIXELS;
    if top < scroll {
        top
    } else if bottom > scroll + LIST_PIXELS {
        (bottom - LIST_PIXELS).min(max_scroll(rows))
    } else {
        scroll
    }
}

/// The list's scroll range for `rows` rows, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 20 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST.3) as usize
}

const fn side(faction: CockpitFaction) -> usize {
    match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    }
}

const fn player_side(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, PERSONNEL_FINDER_WIDTH, PERSONNEL_FINDER_HEIGHT)
}

fn at(frame: egui::Rect, scale: f32, (x, y, w, h): (f32, f32, f32, f32)) -> egui::Rect {
    logical_rect(frame, scale, x, y, w, h)
}

/// Cell colours for SpecForces counts (FUN_00465540): same as the troop
/// finder. Alliance tab → white on red; Imperial tab → black on green.
fn sf_cell_colours(tab: PersonnelTab) -> (egui::Color32, egui::Color32) {
    match tab {
        PersonnelTab::Alliance => (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE),
        PersonnelTab::Imperial => (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK),
    }
}

impl PersonnelFinderState {
    /// Open the Finder on the player's side tab and the Characters view
    /// (`FUN_004637a0`: `FUN_0060d7e0(side, 1)`, `FUN_0060d7e0(3, 1)`). An
    /// open Finder stays as it is.
    pub fn open(&mut self, faction: CockpitFaction) -> bool {
        if self.window.is_some() {
            return false;
        }
        let tab = match faction {
            CockpitFaction::Alliance => PersonnelTab::Alliance,
            CockpitFaction::Empire => PersonnelTab::Imperial,
        };
        self.window = Some(OpenFinder {
            faction,
            tab,
            view: PersonnelView::Characters,
            name: String::new(),
            chosen: None,
            scroll: 0,
            scroll_hold: None,
            focus_name: true,
            last_clicked: None,
        });
        true
    }

    pub fn close(&mut self) {
        self.window = None;
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.window.is_some()
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    pub fn set_tab(&mut self, tab: PersonnelTab) {
        if let Some(window) = &mut self.window {
            if window.tab != tab {
                window.tab = tab;
                window.chosen = None;
                window.scroll = 0;
                window.last_clicked = None;
            }
        }
    }

    pub fn set_view(&mut self, view: PersonnelView) {
        if let Some(window) = &mut self.window {
            if window.view != view {
                window.view = view;
                window.chosen = None;
                window.scroll = 0;
                window.last_clicked = None;
            }
        }
    }
}

/// Draw the open Personnel Finder, if any, and report an action.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_personnel_finder(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    state: &mut PersonnelFinderState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<PersonnelFinderAction> {
    let window = state.window.clone()?;
    let side_idx = side(window.faction);
    let controls = &CONTROLS[side_idx];
    let scale = layout.scale;
    let rect = window_rect(layout);
    let list = rows(
        world,
        fog,
        player_side(window.faction),
        window.tab,
        window.view,
    );

    let mut close = false;
    let mut open_action: Option<PersonnelFinderAction> = None;
    let mut new_side_tab = None;
    let mut new_view_tab = None;
    let mut clicked_row = None;
    let mut name = window.name.clone();
    let mut name_changed = false;
    let mut enter = false;
    let mut wheel = 0.0;
    let mut scroll = window.scroll;
    let mut scroll_hold = window.scroll_hold;

    let id = egui::Id::new("original-personnel-finder");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, id));
    egui::Area::new(id)
        .fixed_pos(rect.min)
        .order(egui::Order::Tooltip)
        // The original's window shows whole at once.
        .fade_in(false)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
            let painter = ui.painter().with_clip_rect(frame);
            let paint = |cache: &mut BmpCache, id: u32, (x, y): (f32, f32)| {
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    id,
                    frame,
                    scale,
                    x,
                    y,
                );
            };

            painter.rect_filled(frame, 0.0, egui::Color32::BLACK);
            // The panel lies under the base, whose rounded corners, keyed
            // on blue, cover the panel's (A0 System, Fleet, Troop and
            // Personnel Finders).
            paint(
                cache,
                PANELS[window.view.index()][window.tab.index()],
                PANEL_AT,
            );
            paint(cache, BACKGROUND[side_idx], (0.0, 0.0));
            paint(cache, RAIL[side_idx], RAIL_AT);

            // Title (0x1890) centred in its 350 in font 5, label (0x1891
            // "Name") in font 4, the tab's label in font 5; the list and the
            // name box in font 10 (A0 2026-10-29).
            let title_font = crate::theme::original_font(5, scale);
            let label_font = crate::theme::original_font(4, scale);
            let font = crate::theme::original_font(10, scale);
            painter.text(
                at(frame, scale, (TITLE_AT.0 + 175.0, TITLE_AT.1, 0.0, 0.0)).min,
                egui::Align2::CENTER_TOP,
                "Personnel Finder",
                title_font.clone(),
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                "Name",
                label_font,
                egui::Color32::WHITE,
            );
            let label_y = window.view.label_y();
            painter.text(
                at(frame, scale, (40.0, label_y, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                window.tab.label(),
                title_font,
                egui::Color32::WHITE,
            );

            // Side tabs.
            for candidate in PersonnelTab::ALL {
                let control = SIDE_TABS[candidate.index()];
                let down = candidate == window.tab;
                if button(
                    ui,
                    cache,
                    at(frame, scale, control.rect),
                    ("personnel-side-tab", candidate.index()),
                    control.art,
                    down,
                    scale,
                ) {
                    new_side_tab = Some(candidate);
                }
            }

            // View tabs (FUN_004637a0 group 0x6e).
            for candidate in PersonnelView::ALL {
                let y_offset = candidate.button_offset();
                let view_rect = (
                    controls.view_origin.0,
                    controls.view_origin.1 + y_offset,
                    controls.view_button_size.0,
                    controls.view_button_size.1,
                );
                let art = VIEW_TAB_ART[side_idx][candidate.index()];
                let down = candidate == window.view;
                if button(
                    ui,
                    cache,
                    at(frame, scale, view_rect),
                    ("personnel-view-tab", candidate.index()),
                    art,
                    down,
                    scale,
                ) {
                    new_view_tab = Some(candidate);
                }
            }

            // Close and Display.
            if button(
                ui,
                cache,
                at(frame, scale, controls.close.rect),
                "personnel-close",
                controls.close.art,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(frame, scale, controls.display.rect),
                "personnel-display",
                controls.display.art,
                false,
                scale,
            ) {
                if let Some(chosen) = window.chosen {
                    if let Some(row) = list.get(chosen) {
                        open_action = Some(action_for_row(row));
                    }
                }
            }

            // Name box (0xcb).
            let name_rect = at(frame, scale, NAME_BOX);
            // port: the caret stays inside the box, as a Windows edit
            // control's does; egui's runs a row past its bottom.
            let outer_clip = ui.clip_rect();
            ui.set_clip_rect(name_rect.intersect(outer_clip));
            let edit = ui.put(
                name_rect,
                egui::TextEdit::singleline(&mut name)
                    .id(egui::Id::new("original-personnel-finder-name"))
                    .frame(false)
                    .font(font.clone())
                    .text_color(egui::Color32::WHITE)
                    .desired_width(name_rect.width()),
            );
            ui.set_clip_rect(outer_clip);
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

            // The list (0xca).
            let list_rect = at(frame, scale, LIST);
            let list_painter = painter.with_clip_rect(list_rect);
            let cell_font = egui::FontId::proportional((10.0 * scale).max(7.0));

            // The bar, then the rows scrolled by pixels and clipped to the
            // list; a row's click area ends at the bar.
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "A list's pixels."
            )]
            {
                scroll = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    frame,
                    scale,
                    crate::list_scroll_bar::RESULTS_ART,
                    LIST,
                    (list.len() * ROW_PIXELS) as f32,
                    scroll as f32,
                    &mut scroll_hold,
                    LIST_STEP,
                )
                .max(0.0) as usize;
            }
            for (offset, row) in list.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
                let line = offset as f32 - scroll as f32 / ROW_HEIGHT;
                if (line + 1.0) * ROW_HEIGHT <= 0.0 || line * ROW_HEIGHT >= LIST.3 {
                    continue;
                }
                // The text keeps the whole row's place; the click area is
                // the row's part in the list.
                let full_row = at(
                    frame,
                    scale,
                    (LIST.0, LIST.1 + ROW_HEIGHT * line, LIST.2 - BAR_WIDTH, ROW_HEIGHT),
                );
                let row_rect = full_row.intersect(list_rect);
                // FUN_004637a0: the text at x 5 (+0xe8) centred on the row
                // (+0xe4 = 0x24), grey 0x787878 and the chosen row white
                // (FUN_00607ea0's colours; A0 2026-10-29).
                let name_colour = if window.chosen == Some(offset) {
                    egui::Color32::WHITE
                } else {
                    ROW_COLOR
                };

                match row {
                    PersonnelFinderRow::Character { name, .. } => {
                        list_painter.text(
                            full_row.left_center() + egui::vec2(ROW_TEXT_X * scale, 0.0),
                            egui::Align2::LEFT_CENTER,
                            name,
                            font.clone(),
                            name_colour,
                        );
                    }
                    PersonnelFinderRow::SpecForces { name, counts, .. } => {
                        list_painter.text(
                            full_row.left_center() + egui::vec2(ROW_TEXT_X * scale, 0.0),
                            egui::Align2::LEFT_CENTER,
                            name,
                            font.clone(),
                            name_colour,
                        );
                        // Four columns at x = 222 + 28*i (FUN_00465540).
                        let (box_colour, text_on_box) = sf_cell_colours(window.tab);
                        for (i, cell) in counts.iter().enumerate() {
                            let cell_x = 222.0 + 28.0 * i as f32;
                            if cell.count == 0 {
                                let cell_pos = at(
                                    frame,
                                    scale,
                                    (LIST.0 + cell_x, LIST.1 + ROW_HEIGHT * line + 2.0, 28.0, 0.0),
                                )
                                .min;
                                list_painter.text(
                                    cell_pos + egui::vec2(10.0 * scale, 0.0),
                                    egui::Align2::CENTER_TOP,
                                    "-",
                                    cell_font.clone(),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                let box_rect = at(
                                    frame,
                                    scale,
                                    (
                                        LIST.0 + cell_x,
                                        LIST.1 + ROW_HEIGHT * line + 2.0,
                                        21.0,
                                        16.0,
                                    ),
                                );
                                list_painter.rect_filled(box_rect, 0.0, box_colour);
                                list_painter.text(
                                    box_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    cell.count.to_string(),
                                    cell_font.clone(),
                                    text_on_box,
                                );
                            }
                        }
                    }
                }

                let response = ui.interact(
                    row_rect,
                    ui.id().with(("personnel-row", offset)),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    let (now, delay) = (
                        ui.input(|input| input.time),
                        ctx.options(|options| options.input_options.max_double_click_delay),
                    );
                    if window
                        .last_clicked
                        .is_some_and(|(last, at_time)| last == offset && now - at_time <= delay)
                    {
                        open_action = Some(action_for_row(row));
                    } else {
                        clicked_row = Some((offset, now));
                    }
                }
            }
            let pointer = ui.input(|input| input.pointer.hover_pos());
            if pointer.is_some_and(|point| rect_contains(list_rect, point)) {
                wheel = ui.input(|input| input.raw_scroll_delta.y);
            }
        });

    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        open_window.name.clone_from(&name);
        let choice = prefix_choice(&list, &name);
        open_window.chosen = choice;
        if let Some(index) = choice {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at_time)) = clicked_row {
        open_window.last_clicked = Some((row, at_time));
        open_window.chosen = Some(row);
        // Copy the display name into the name box.
        open_window.name = list[row].display_name().to_owned();
    }
    if enter {
        if let Some(chosen) = open_window.chosen {
            if let Some(row) = list.get(chosen) {
                open_action = Some(action_for_row(row));
            }
        }
    }
    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        open_window.scroll = if wheel > 0.0 {
            open_window.scroll.saturating_sub(ROW_PIXELS)
        } else {
            (open_window.scroll + ROW_PIXELS).min(max_scroll(list.len()))
        };
    }
    if let Some(tab) = new_side_tab {
        state.set_tab(tab);
    }
    if let Some(view) = new_view_tab {
        state.set_view(view);
    }
    if let Some(action) = open_action {
        state.close();
        return Some(action);
    }
    if close || escape {
        state.close();
    }
    None
}

fn action_for_row(row: &PersonnelFinderRow) -> PersonnelFinderAction {
    match row {
        PersonnelFinderRow::Character { key, system, .. } => PersonnelFinderAction::OpenCharacter {
            character: *key,
            system: *system,
        },
        PersonnelFinderRow::SpecForces { system, fleet, .. } => {
            if let Some(fleet) = fleet {
                PersonnelFinderAction::OpenFleet {
                    system: *system,
                    fleet: *fleet,
                }
            } else {
                PersonnelFinderAction::OpenSystem { system: *system }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::CockpitViewport;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::DatId;
    use rebellion_core::world::{
        Character, ControlKind, Sector, SkillPair, SpecialForceUnit, System,
    };

    fn layout() -> CockpitLayout {
        CockpitLayout {
            canvas: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 480.0,
            },
            galaxy: CockpitViewport {
                x: 65.0,
                y: 60.0,
                width: 485.0,
                height: 350.0,
            },
            scale: 1.0,
        }
    }

    fn add_system(world: &mut GameWorld, name: &str) -> SystemKey {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(36),
            name: "Sesswenna".into(),
            group: SectorGroup::Core,
            x: 317,
            y: 248,
            systems: Vec::new(),
        });
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(100),
            name: name.into(),
            sector,
            x: 320,
            y: 250,
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
            control: ControlKind::Controlled(Faction::Alliance),
        })
    }

    fn default_character() -> Character {
        Character {
            dat_id: DatId::new(0x3000_0001),
            name: String::new(),
            is_alliance: true,
            is_empire: false,
            is_major: true,
            diplomacy: SkillPair {
                base: 50,
                variance: 0,
            },
            espionage: SkillPair {
                base: 50,
                variance: 0,
            },
            ship_design: SkillPair {
                base: 50,
                variance: 0,
            },
            troop_training: SkillPair {
                base: 50,
                variance: 0,
            },
            facility_design: SkillPair {
                base: 50,
                variance: 0,
            },
            combat: SkillPair {
                base: 50,
                variance: 0,
            },
            leadership: SkillPair {
                base: 50,
                variance: 0,
            },
            loyalty: SkillPair {
                base: 50,
                variance: 0,
            },
            jedi_probability: 0,
            jedi_level: SkillPair {
                base: 0,
                variance: 0,
            },
            can_be_admiral: false,
            can_be_commander: false,
            can_be_general: false,
            is_unable_to_betray: false,
            is_jedi_trainer: false,
            is_known_jedi: false,
            hyperdrive_modifier: 0,
            enhanced_loyalty: 0,
            on_mission: false,
            on_hidden_mission: false,
            on_mandatory_mission: false,
            captured_by: None,
            capture_tick: None,
            is_captive: false,
            current_system: None,
            current_fleet: None,
            heritage_known: false,
            is_killed: false,
            is_retired: false,
            recruited: true,
            injury: 0,
            force_potential: false,
            force_aware: false,
            force_base: 0,
            force_discoverer: None,
            force_training: 0,
            current_abode: None,
            dagobah_completed: false,
            encounter: None,
            encounter_day: 0,
            escape_barred: false,
            adrift: false,
            bounty_captured: false,
            command: rebellion_core::character_command::CommandKind::None,
            command_names: Default::default(),
            ship: None,
        }
    }

    fn add_sf(
        world: &mut GameWorld,
        system: SystemKey,
        class_raw: u32,
        is_alliance: bool,
    ) -> SpecialForceKey {
        let key = world.special_forces.insert(SpecialForceUnit {
            class_dat_id: DatId::new(class_raw),
            is_alliance,
            skills: [0; 8],
            on_mission: false,
        });
        world.systems[system].special_forces.push(key);
        key
    }

    #[test]
    fn the_specforces_view_lists_systems_holding_the_tabs_sf_units() {
        // FUN_00464e10: walk systems, check for SF of the tab's side.
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        add_sf(&mut world, yavin, 0x3c00_0004, true);
        add_sf(&mut world, yavin, 0x3c00_0004, true);
        add_sf(&mut world, yavin, 0x3c00_0002, true);
        let fog = FogState::new(Faction::Alliance);

        let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);

        assert_eq!(listed.len(), 1);
        if let PersonnelFinderRow::SpecForces { counts, .. } = &listed[0] {
            assert_eq!(counts[0].count, 2, "first SF class column");
            assert_eq!(counts[1].count, 1, "second SF class column");
            assert_eq!(counts[2].count, 0);
            assert_eq!(counts[3].count, 0);
        } else {
            panic!("expected SpecForces row");
        }
    }

    #[test]
    fn a_fleet_carrying_the_tabs_sf_units_is_a_row_of_its_own() {
        // FUN_00464e10: after a system's own row, each fleet of the tab's
        // side there whose ships carry special forces gets a row
        // (FUN_00442130 with the fleet), counted over the fleet
        // (FUN_00465540, type 8).
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        let unit = add_sf(&mut world, yavin, 0x3c00_0002, true);
        world.systems[yavin].special_forces.clear();
        let fleet = world.insert_fleet(rebellion_core::world::Fleet {
            location: yavin,
            capital_ships: Vec::new(),
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
            special_forces: vec![unit],
        });
        world.systems[yavin].fleets.push(fleet);
        let fog = FogState::new(Faction::Alliance);

        let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert_eq!(listed.len(), 1);
        let PersonnelFinderRow::SpecForces {
            name,
            system,
            fleet: row_fleet,
            counts,
        } = &listed[0]
        else {
            panic!("expected a SpecForces row");
        };
        assert_eq!(
            Some(name.clone()),
            crate::system_window::fleet_label(&world, fleet)
        );
        assert_eq!((*system, *row_fleet), (yavin, Some(fleet)));
        assert_eq!(counts[1].count, 1);
        // The other side's tab lists nothing.
        assert!(
            specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).is_empty()
        );
    }

    #[test]
    fn the_characters_view_lists_recruited_living_characters_of_the_tabs_side() {
        // FUN_00464e10 view==2: walk characters of the side, call
        // FUN_00465bb0 for each.
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");

        let mut luke = default_character();
        luke.name = "Luke Skywalker".into();
        luke.is_alliance = true;
        luke.current_system = Some(yavin);
        luke.recruited = true;
        world.characters.insert(luke);

        let mut vader = default_character();
        vader.name = "Darth Vader".into();
        vader.is_alliance = false;
        vader.is_empire = true;
        vader.current_system = Some(yavin);
        vader.recruited = true;
        world.characters.insert(vader);

        // Not recruited: hidden.
        let mut hidden = default_character();
        hidden.name = "Hidden".into();
        hidden.is_alliance = true;
        hidden.recruited = false;
        world.characters.insert(hidden);

        let fog = FogState::new(Faction::Alliance);

        let alliance = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert_eq!(alliance.len(), 1);
        assert!(alliance[0].display_name().starts_with("Luke Skywalker"));

        let imperial = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial);
        assert_eq!(imperial.len(), 1);
        assert!(imperial[0].display_name().starts_with("Darth Vader"));
    }

    #[test]
    fn display_opens_the_fleet_the_missions_or_the_defenses_window() {
        // FUN_00429440 family 0x30: a fleet parent opens kind 4; at a
        // system, a mission that is not hidden (FUN_00520b70 == 0) opens
        // kind 11, anything else kind 10. Manual p. 100: "system defenses,
        // fleet or mission".
        let mut character = default_character();
        assert_eq!(character_target(&character), CharacterTarget::Defenses);
        character.on_mission = true;
        assert_eq!(character_target(&character), CharacterTarget::Missions);
        character.on_hidden_mission = true;
        assert_eq!(character_target(&character), CharacterTarget::Defenses);
        let fleet = FleetKey::default();
        character.current_fleet = Some(fleet);
        character.on_hidden_mission = false;
        assert_eq!(character_target(&character), CharacterTarget::Fleet(fleet));
    }

    #[test]
    fn a_character_on_mission_shows_its_state_in_the_name() {
        // FUN_00465bb0: "Name - Location ( On Mission ) ".
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");

        let mut luke = default_character();
        luke.name = "Luke Skywalker".into();
        luke.current_system = Some(yavin);
        luke.on_mission = true;
        world.characters.insert(luke);

        let fog = FogState::new(Faction::Alliance);
        let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert!(listed[0].display_name().contains("On Mission"));
    }

    #[test]
    fn a_killed_character_is_listed_as_killed() {
        // FUN_00465bb0's destroyed branch; the reference capture lists
        // "Ackbar - Killed" (steam-guide guide-023.png).
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");

        let mut luke = default_character();
        luke.name = "Luke Skywalker".into();
        luke.current_system = Some(yavin);
        luke.is_killed = true;
        world.characters.insert(luke);

        let fog = FogState::new(Faction::Alliance);
        let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert_eq!(listed[0].display_name(), "Luke Skywalker - Killed");
    }

    #[test]
    fn a_retired_character_is_listed_as_retired() {
        // FUN_00465bb0: destruction reason 0x14..0x16 reads 0x8799
        // "Retired"; a Retire order destroys with 0x14 (FUN_00534c00).
        let mut world = GameWorld::default();
        let mut han = default_character();
        han.name = "Han Solo".into();
        han.is_killed = true;
        han.is_retired = true;
        world.characters.insert(han);

        let fog = FogState::new(Faction::Alliance);
        let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert_eq!(listed[0].display_name(), "Han Solo - Retired");
    }

    #[test]
    fn a_rows_state_word_sits_in_spaced_brackets_and_captured_comes_first() {
        // FUN_00465bb0: " ( " + the first of Captured, On Mission, Injured,
        // Enroute + " ) " (DAT_006a8790, DAT_006a878c).
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        let mut leia = default_character();
        leia.name = "Leia".into();
        leia.current_system = Some(yavin);
        leia.on_mission = true;
        leia.is_captive = true;
        world.characters.insert(leia);
        let mut han = default_character();
        han.name = "Han".into();
        world.characters.insert(han);
        // Aboard a fleet is not en route, wherever the fleet is.
        let mut wedge = default_character();
        wedge.name = "Wedge".into();
        wedge.current_fleet = Some(rebellion_core::ids::FleetKey::default());
        world.characters.insert(wedge);

        let fog = FogState::new(Faction::Alliance);
        let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        assert_eq!(
            rows(
                &world,
                &fog,
                Faction::Alliance,
                PersonnelTab::Alliance,
                PersonnelView::Characters
            ),
            listed
        );
        let names: Vec<&str> = listed
            .iter()
            .map(PersonnelFinderRow::display_name)
            .collect();
        assert_eq!(
            names,
            [
                "Han - Location Unknown ( Enroute ) ",
                "Leia - Yavin ( Captured ) ",
                "Wedge - Location Unknown"
            ]
        );
    }

    #[test]
    fn a_row_adds_injury_and_command_and_an_abode_names_only_the_place() {
        // FUN_00465bb0: Injured (0x8747, +0x94) after Captured and On
        // Mission; the command word (+0x96: 1 Commander 0x8802, 2 Admiral
        // 0x8801, 3 General 0x8800) after the state; an abode (family
        // 0x98) gives its name and nothing more.
        use rebellion_core::character_command::CommandKind;
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        let jail = world.abodes.insert(rebellion_core::world::Abode {
            dat_id: rebellion_core::ids::DatId::new(0x9800_0480),
            name: "Empire Jail".into(),
            side: Faction::Empire,
            special_forces: Vec::new(),
        });
        let mut ackbar = default_character();
        ackbar.name = "Ackbar".into();
        ackbar.current_system = Some(yavin);
        ackbar.injury = 3;
        ackbar.command = CommandKind::Admiral;
        world.characters.insert(ackbar);
        let mut dodonna = default_character();
        dodonna.name = "Dodonna".into();
        dodonna.current_system = Some(yavin);
        dodonna.command = CommandKind::General;
        world.characters.insert(dodonna);
        let mut leia = default_character();
        leia.name = "Leia".into();
        leia.current_abode = Some(jail);
        leia.is_captive = true;
        leia.command = CommandKind::Commander;
        world.characters.insert(leia);

        let fog = FogState::new(Faction::Alliance);
        let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        let names: Vec<&str> = listed
            .iter()
            .map(PersonnelFinderRow::display_name)
            .collect();
        assert_eq!(
            names,
            [
                "Ackbar - Yavin ( Injured )  ( Admiral ) ",
                "Dodonna - Yavin ( General ) ",
                "Leia - Empire Jail"
            ]
        );
    }

    #[test]
    fn rows_are_sorted_by_name_ignoring_case() {
        // FUN_005f59f0: rows inserted in name order.
        let mut world = GameWorld::default();
        let beta = add_system(&mut world, "beta");
        let alpha = add_system(&mut world, "Alpha");
        add_sf(&mut world, beta, 0x3c00_0004, true);
        add_sf(&mut world, alpha, 0x3c00_0004, true);
        let fog = FogState::new(Faction::Alliance);

        let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
        let names: Vec<&str> = listed.iter().map(|r| r.display_name()).collect();
        assert_eq!(names, ["Alpha", "beta"]);
    }

    #[test]
    fn a_typed_name_chooses_the_first_row_it_best_begins() {
        // FUN_00609650: prefix match across both row kinds.
        let rows = vec![
            PersonnelFinderRow::SpecForces {
                name: "Coruscant".into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [SfCount::default(); 4],
            },
            PersonnelFinderRow::SpecForces {
                name: "Yavin".into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [SfCount::default(); 4],
            },
        ];
        assert_eq!(prefix_choice(&rows, "ya"), Some(1));
        assert_eq!(prefix_choice(&rows, "cor"), Some(0));
        // FUN_00609650: with no row sharing the first letter, the first.
        assert_eq!(prefix_choice(&rows, "z"), Some(0));
        assert_eq!(prefix_choice(&rows, ""), Some(0));
    }

    #[test]
    fn the_action_for_a_character_row_opens_character_and_sf_row_opens_system() {
        // FUN_00429440: character → family 0x30, system → family 0x90.
        let system = SystemKey::default();
        let character = CharacterKey::default();

        let char_row = PersonnelFinderRow::Character {
            key: character,
            name: "Luke - Yavin".into(),
            location: "Yavin".into(),
            state: String::new(),
            system: Some(system),
        };
        assert_eq!(
            action_for_row(&char_row),
            PersonnelFinderAction::OpenCharacter {
                character,
                system: Some(system),
            }
        );

        let sf_row = PersonnelFinderRow::SpecForces {
            name: "Yavin".into(),
            system,
            fleet: None,
            counts: [SfCount::default(); 4],
        };
        assert_eq!(
            action_for_row(&sf_row),
            PersonnelFinderAction::OpenSystem { system }
        );
    }

    #[test]
    fn scrolled_rows_clip_to_the_list_and_keep_their_centres() {
        // Twelve characters scrolled 30 pixels: row 0 (-30..-10) is gone,
        // row 1 (-10..10) centres on the list's top, row 9 (150..170) shows
        // its upper part, row 10 (170..) nothing. A press in the bar's
        // column chooses nothing; one at x 370 chooses row 4; a wheel notch
        // moves a row (port).
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        for index in 0..12 {
            let mut character = default_character();
            character.name = format!("C{index:02}");
            character.is_alliance = true;
            character.current_system = Some(yavin);
            character.recruited = true;
            world.characters.insert(character);
        }
        let fog = FogState::new(Faction::Alliance);
        let origin = window_rect(layout()).min;
        let run = |state: &mut PersonnelFinderState, at: egui::Pos2, extra: Vec<egui::Event>| {
            let ctx = egui::Context::default();
            crate::theme::load_fonts(&ctx);
            let mut cache = BmpCache::new();
            let mut shapes = Vec::new();
            let frames = [vec![], vec![]].into_iter().chain(extra.into_iter().map(|event| vec![event]));
            for events in frames {
                let mut input = vec![egui::Event::PointerMoved(at)];
                input.extend(events);
                shapes = ctx
                    .run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                            events: input,
                            ..Default::default()
                        },
                        |ctx| {
                            let _ = draw_personnel_finder(ctx, &world, &fog, state, layout(), &mut cache);
                        },
                    )
                    .shapes;
            }
            shapes
        };
        let press = |at: egui::Pos2, pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let opened = || {
            let mut state = PersonnelFinderState::default();
            state.open(CockpitFaction::Alliance);
            state.window.as_mut().unwrap().scroll = 30;
            state
        };

        let mut state = opened();
        let in_bar = origin + egui::vec2(380.0, LIST.1 + 50.0);
        let shapes = run(&mut state, in_bar, vec![press(in_bar, true), press(in_bar, false)]);
        let centre = |name: &str| {
            shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with(name) => {
                    Some(text.pos.y + text.galley.size().y / 2.0 - origin.y)
                }
                _ => None,
            })
        };
        assert_eq!(centre("C00 "), None);
        assert!((centre("C01 ").unwrap() - LIST.1).abs() < 0.01);
        assert!((centre("C09 ").unwrap() - (LIST.1 + 160.0)).abs() < 0.01);
        assert_eq!(centre("C10 "), None);
        assert_eq!(state.window.as_ref().unwrap().chosen, None);

        let mut state = opened();
        let near_bar = origin + egui::vec2(370.0, LIST.1 + 50.0);
        let _ = run(&mut state, near_bar, vec![press(near_bar, true), press(near_bar, false)]);
        assert_eq!(state.window.as_ref().unwrap().chosen, Some(4));

        let wheel = |delta: f32| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, delta),
            modifiers: egui::Modifiers::default(),
        };
        let mut state = opened();
        let _ = run(&mut state, near_bar, vec![wheel(-1.0)]);
        assert_eq!(state.window.as_ref().unwrap().scroll, 50);
        let _ = run(&mut state, near_bar, vec![wheel(1.0)]);
        assert_eq!(state.window.as_ref().unwrap().scroll, 30);
    }

    #[test]
    fn eight_whole_rows_fit_the_list() {
        // FUN_00607ea0: list 161 tall, rows 20 tall.
        assert_eq!(LIST_PIXELS / ROW_PIXELS, 8);
    }

    #[test]
    fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
        // By pixels (FUN_0060a490): row 8 (160..180) needs 19; twenty rows
        // reach 400 + 5 - 161 = 244 (FUN_00609ae0).
        assert_eq!(scrolled_to(0, 7, 20), 0);
        assert_eq!(scrolled_to(0, 8, 20), 19);
        assert_eq!(scrolled_to(60, 1, 20), 20);
        assert_eq!(scrolled_to(60, 10, 20), 60);
        assert_eq!(scrolled_to(40, 10, 20), 59);
        assert_eq!(max_scroll(20), 244);
        assert_eq!(max_scroll(8), 0);
    }

    #[test]
    fn the_longest_common_beginning_wins_and_a_tie_keeps_the_first() {
        // FUN_00609650 keeps a row only for a strictly longer match.
        let rows: Vec<PersonnelFinderRow> = ["Corulag", "Coruscant", "Yavin"]
            .into_iter()
            .map(|name| PersonnelFinderRow::SpecForces {
                name: name.into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [SfCount::default(); 4],
            })
            .collect();
        assert_eq!(prefix_choice(&rows, "corus"), Some(1));
        assert_eq!(prefix_choice(&rows, "cor"), Some(0));
    }

    #[test]
    fn each_side_tab_names_its_side_and_paints_its_own_cells() {
        // TEXTSTRA 0x1893/0x1894; FUN_00465540's brushes as the Troop
        // Finder's: red with white text, green with black.
        assert_eq!(PersonnelTab::Alliance.label(), "Alliance Personnel");
        assert_eq!(PersonnelTab::Imperial.label(), "Imperial Personnel");
        assert_eq!(
            sf_cell_colours(PersonnelTab::Alliance),
            (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE)
        );
        assert_eq!(
            sf_cell_colours(PersonnelTab::Imperial),
            (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK)
        );
    }

    #[test]
    fn opening_an_open_finder_keeps_its_state() {
        // FUN_0042a4d0 builds the window only when no type 0x17 is open.
        let mut state = PersonnelFinderState::default();
        assert!(state.open(CockpitFaction::Alliance));
        state.set_tab(PersonnelTab::Imperial);

        assert!(!state.open(CockpitFaction::Alliance));
        assert_eq!(state.window.as_ref().unwrap().tab, PersonnelTab::Imperial);
        assert!(state.is_open());
        state.close();
        assert!(!state.is_open());
    }

    #[test]
    fn a_new_side_tab_clears_the_choice() {
        // FUN_00464e10: tab switch rebuilds the list.
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().chosen = Some(0);

        state.set_tab(PersonnelTab::Imperial);
        assert_eq!(state.window.as_ref().unwrap().chosen, None);
    }

    #[test]
    fn a_new_view_tab_clears_the_choice() {
        // FUN_00464e10: view switch rebuilds the list.
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().chosen = Some(0);

        state.set_view(PersonnelView::SpecForces);
        assert_eq!(state.window.as_ref().unwrap().chosen, None);
    }

    #[test]
    fn the_finder_opens_with_the_characters_view_and_players_side_tab() {
        // FUN_004637a0: FUN_0060d7e0(+0x148, 3, 1), view 3, the Characters
        // button (TEXTSTRA 0x1899); FUN_0060d7e0(+0x144, side, 1), the
        // player's side.
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.view, PersonnelView::Characters);
        assert_eq!(window.tab, PersonnelTab::Alliance);

        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Empire);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.view, PersonnelView::Characters);
        assert_eq!(window.tab, PersonnelTab::Imperial);
    }

    #[test]
    fn escape_closes_the_finder() {
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        let world = GameWorld::default();
        let fog = FogState::new(Faction::Alliance);
        let mut cache = BmpCache::new();
        let layout = layout();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            draw_personnel_finder(ctx, &world, &fog, &mut state, layout, &mut cache);
        });

        assert!(!state.is_open());
    }

    #[test]
    fn the_title_and_label_take_the_originals_text_fonts_and_places() {
        // FUN_004637a0: the title centred in (36, 14, 350) in font 5, the
        // label TEXTSTRA 0x1891 "Name" in font 4 (A0 2026-10-29).
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        let world = GameWorld::default();
        let fog = FogState::new(Faction::Alliance);
        let mut cache = BmpCache::new();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_personnel_finder(ctx, &world, &fog, &mut state, layout(), &mut cache);
                },
            );
            shapes = output.shapes;
        }
        let find = |wanted: &str| {
            shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == wanted => Some((
                    text.pos,
                    text.galley.size(),
                    text.galley.job.sections[0].format.font_id.size,
                )),
                _ => None,
            })
        };
        let origin = window_rect(layout()).min;
        let (pos, size, font) = find("Personnel Finder").unwrap();
        assert!((pos.x + size.x / 2.0 - origin.x - 211.0).abs() < 0.01, "{pos:?}");
        assert_eq!(font, crate::theme::original_font(5, 1.0).size);
        let (pos, _, font) = find("Name").unwrap();
        assert_eq!(pos, origin + egui::vec2(36.0, 48.0));
        assert_eq!(font, crate::theme::original_font(4, 1.0).size);
        assert!(find("Character Name").is_none());
    }

    #[test]
    fn rows_sit_at_x_five_centred_grey_and_the_chosen_one_white_at_any_scale() {
        // FUN_004637a0: +0xe8 = 5, +0xe4 = 0x24; FUN_00607ea0's colours.
        // At scale 2: Luke's row and, in the special forces view, Yavin's.
        let mut world = GameWorld::default();
        let yavin = add_system(&mut world, "Yavin");
        let mut luke = default_character();
        luke.name = "Luke Skywalker".into();
        luke.is_alliance = true;
        luke.current_system = Some(yavin);
        luke.recruited = true;
        world.characters.insert(luke);
        add_sf(&mut world, yavin, 0x3c00_0001, true);
        let fog = FogState::new(Faction::Alliance);
        let layout = CockpitLayout {
            scale: 2.0,
            canvas: crate::cockpit::CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 960.0,
            },
            galaxy: crate::cockpit::CockpitViewport {
                x: 130.0,
                y: 120.0,
                width: 970.0,
                height: 700.0,
            },
        };
        let origin = window_rect(layout).min;
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        let row = |state: &mut PersonnelFinderState| {
            let ctx = egui::Context::default();
            crate::theme::load_fonts(&ctx);
            let mut cache = BmpCache::new();
            let mut shapes = Vec::new();
            for _ in 0..2 {
                shapes = ctx
                    .run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1400.0, 1100.0),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            let _ = draw_personnel_finder(ctx, &world, &fog, state, layout, &mut cache);
                        },
                    )
                    .shapes;
            }
            shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text)
                        if (text.pos.x - origin.x - (LIST.0 + 5.0) * 2.0).abs() < 0.01 =>
                    {
                        Some((
                            text.galley.text().to_owned(),
                            text.pos.y + text.galley.size().y / 2.0 - origin.y,
                            text.galley.job.sections[0].format.color,
                        ))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let (text, middle, colour) = row(&mut state);
        assert!(text.starts_with("Luke Skywalker"), "{text}");
        assert!((middle - (LIST.1 + 10.0) * 2.0).abs() < 0.01, "{middle}");
        assert_eq!(colour, ROW_COLOR);
        state.window.as_mut().unwrap().chosen = Some(0);
        assert_eq!(row(&mut state).2, egui::Color32::WHITE);
        let window = state.window.as_mut().unwrap();
        window.view = PersonnelView::SpecForces;
        window.chosen = None;
        let (text, middle, colour) = row(&mut state);
        assert!(text.starts_with("Yavin"), "{text}");
        assert!((middle - (LIST.1 + 10.0) * 2.0).abs() < 0.01, "{middle}");
        assert_eq!(colour, ROW_COLOR);
    }

    #[test]
    fn the_finder_blocks_only_its_own_rectangle() {
        let mut state = PersonnelFinderState::default();
        let rect = window_rect(layout());
        assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

        state.open(CockpitFaction::Alliance);
        assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
        assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
    }

    #[test]
    fn count_sf_tallies_by_class_for_the_tabs_side() {
        // FUN_00465540: four columns, each the count of its DatId.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yavin");
        let k1 = add_sf(&mut world, system, 0x3c00_0004, true); // col 0
        let k2 = add_sf(&mut world, system, 0x3c00_0004, true); // col 0
        let k3 = add_sf(&mut world, system, 0x3c00_0003, true); // col 3
        let _k4 = add_sf(&mut world, system, 0x3c00_0007, false); // empire, ignored

        let keys = vec![k1, k2, k3];
        let counts = count_sf(&world, &keys, PersonnelTab::Alliance);
        assert_eq!(counts[0].count, 2);
        assert_eq!(counts[1].count, 0);
        assert_eq!(counts[3].count, 1);
        assert!(counts[0].all_usable);
    }

    #[test]
    fn each_view_has_its_own_panel_and_button() {
        // FUN_004637a0 / FUN_00464e10: view 3, Characters, shows 10536 or
        // 10537 and its face button 10542 sits at the top; view 2,
        // SpecForces, shows 10538 or 10539 (the column icons) and its
        // soldiers button 10546 sits 0x36 lower.
        let characters = PersonnelView::Characters.index();
        let specforces = PersonnelView::SpecForces.index();
        assert_eq!(PANELS[characters], [10536, 10537]);
        assert_eq!(PANELS[specforces], [10538, 10539]);
        assert_eq!(VIEW_TAB_ART[0][characters], (10542, 10543));
        assert_eq!(VIEW_TAB_ART[1][specforces], (10548, 10549));
        assert!(PersonnelView::Characters.button_offset().abs() < f32::EPSILON);
        assert!((PersonnelView::SpecForces.button_offset() - 54.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_tab_label_y_differs_by_view() {
        // FUN_00464e10: view 2, SpecForces, y = 0x77 = 119; else 0x73 = 115.
        assert!((PersonnelView::SpecForces.label_y() - 119.0).abs() < f32::EPSILON);
        assert!((PersonnelView::Characters.label_y() - 115.0).abs() < f32::EPSILON);
    }
}
