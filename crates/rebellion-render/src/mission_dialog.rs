//! The original mission-creation dialog (`FUN_0046a750` builds it,
//! `FUN_0046a9c0` its controls, `FUN_0046c3c0` handles them;
//! `ghidra/notes/mission-dialog.md`).
//!
//! A 259 by 355 window with two tabbed pages. The first chooses the mission
//! kind and shows the target; the second splits the chosen characters and
//! special forces between agents and decoys. "Begin Mission" submits the
//! order, "Cancel" and the close box discard it, and "Encylopedia" opens the
//! Encyclopedia.

use egui_macroquad::egui;
use rebellion_core::missions::{
    MissionFaction, MissionKind, MissionMember, MissionTarget, MissionTargetPick,
};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::CockpitLayout;
use crate::sector_window::planet_resource_id;
use crate::system_window::{
    character_picture, exact_clicked, logical_rect, rect_contains, special_force_mini,
};

pub const MISSION_DIALOG_WIDTH: f32 = 259.0;
pub const MISSION_DIALOG_HEIGHT: f32 = 355.0;

// STRATEGY bitmaps (FUN_0046a9c0).
const TITLE_ALLIANCE: u32 = 10801;
const TITLE_EMPIRE: u32 = 10802;
const PANEL_MISSION: u32 = 11100;
const PANEL_AGENTS: u32 = 11101;
const AGENTS_HEADER: [u32; 2] = [11121, 11123];
const DECOYS_HEADER: [u32; 2] = [11122, 11124];
const CLOSE: (u32, u32) = (10108, 10109);
const MISSIONS: (u32, u32) = (10606, 10607);
const CANCEL: (u32, u32) = (10596, 10597);
const BEGIN: (u32, u32) = (10594, 10595);
const ENCYCLOPEDIA: (u32, u32) = (10592, 10593);
const TO_DECOYS: (u32, u32) = (11117, 11118);
const TO_AGENTS: (u32, u32) = (11119, 11120);
const MISSION_TAB: [(u32, u32); 2] = [(11103, 11104), (11105, 11106)];
const AGENTS_TAB: [(u32, u32); 2] = [(11107, 11108), (11109, 11110)];

// TEXTSTRA strings.
/// TEXTSTRA 34055, the title of both pages.
const CREATE_MISSION: &str = "Create Mission";
/// TEXTSTRA 34052, the target label on the first page.
const TARGET: &str = "Target";

// Geometry (FUN_0046a9c0), in dialog pixels.
const KIND_BOX: (f32, f32, f32, f32) = (35.0, 62.0, 200.0, 113.0);
const AGENTS_LIST: (f32, f32) = (8.0, 93.0);
const DECOYS_LIST: (f32, f32) = (136.0, 93.0);
const LIST_SIZE: (f32, f32) = (108.0, 213.0);
const ITEM_SIZE: (f32, f32) = (107.0, 59.0);
/// The target art is centered in a 165 by 79 box at (51, 211).
const TARGET_BOX: (f32, f32, f32, f32) = (51.0, 211.0, 165.0, 79.0);
/// The target's name, centered in a 185-wide box at (37, 294).
const TARGET_NAME: (f32, f32, f32) = (37.0, 294.0, 185.0);

/// A list item's name color, unselected and selected (list `+0xd8`/`+0xdc`,
/// `FUN_00607ea0`).
const ITEM_TEXT: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
/// A list item's name font: the regular 16-pixel cell, font 4 (13 ppem).
/// A0 2026-11-16-partial-cells/r-3 (sha256 15244144bb9c) starts the
/// Imperial Probe Droid's two lines 16 rows apart, and its glyphs, like
/// "Han Solo" in 2026-11-15-partial-cells/rescue-agents-tab-han-d763
/// (b667575fbb6b), are taller and thinner than the Defenses window's
/// font 10.
const ITEM_FONT: u16 = 4;
const ITEM_TEXT_SELECTED: egui::Color32 = egui::Color32::WHITE;
/// The width an item's name breaks within (`FUN_00601b80`, `DrawTextA`
/// with `DT_WORDBREAK`): the item's. A0 r-3 breaks "Imperial Probe Droid"
/// after "Probe", both lines centred on the item.
const ITEM_TEXT_WIDTH: f32 = ITEM_SIZE.0;

/// An item's name as `DT_WORDBREAK` lays it in [`ITEM_TEXT_WIDTH`].
fn item_name_lines(name: &str) -> Vec<String> {
    crate::theme::gdi_word_break(name, ITEM_TEXT_WIDTH, |line| {
        crate::theme::gdi_pen_positions(ITEM_FONT, line).map_or(0.0, |(_, width)| width)
    })
}
/// A mission kind's name color in the box and its drop-down: the box's
/// `+0x94`, white (`FUN_0060bed0`, set again by `FUN_0046a9c0`), which
/// `FUN_0060c210` draws the shown item's name in. The item's own `+0x60`
/// gray is not the one shown; A0 menus2/11-13 show the name white.
const KIND_TEXT: egui::Color32 = egui::Color32::WHITE;
/// The drop-down's other kinds keep the item's own `+0x60`, gray (A0
/// kinds/31: "Espionage" under the shown "Diplomacy").
const LIST_KIND_TEXT: egui::Color32 = egui::Color32::from_rgb(0x80, 0x80, 0x80);
/// The title, "Target" and the target's name: `FUN_0046a9c0` sets font 5
/// (`FUN_00601c60(5)`), bold Arial at 13 ppem, before each. A0
/// menus2/11 draws the title 91 pixels wide, its capitals 10 tall.
const TEXT_FONT: u16 = 5;

/// The two tabs of control `0x96` (`FUN_0046c8a0`, page at `+0x118`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissionDialogPage {
    /// Tab `0x97`: the mission kind and the target.
    #[default]
    Mission,
    /// Tab `0x98`: agents and decoys.
    Agents,
}

/// One open dialog: the order it edits and its controls' state.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionDialog {
    faction: MissionFaction,
    /// The object the order's release targeted, and its system.
    target: MissionTargetPick,
    /// The kinds `FUN_004f5380` listed when the dialog was built.
    kinds: Vec<MissionKind>,
    kind: usize,
    page: MissionDialogPage,
    /// The mission-kind drop-down is open (`CoolSelectionBoxClass +0xe8`).
    list_open: bool,
    /// The drop-down list's first visible pixel: it scrolls by pixels on
    /// its bar (A0 kinds/23).
    list_scroll: f32,
    list_hold: Option<crate::list_scroll_bar::Hold>,
    agents: Vec<MissionMember>,
    decoys: Vec<MissionMember>,
    selected: Vec<MissionMember>,
    agents_row: usize,
    decoys_row: usize,
}

impl MissionDialog {
    #[must_use]
    pub fn kind(&self) -> MissionKind {
        self.kinds[self.kind]
    }

    /// The kinds the dialog lists.
    #[must_use]
    pub fn kinds(&self) -> &[MissionKind] {
        &self.kinds
    }

    /// The target the order's release set.
    #[must_use]
    pub fn target(&self) -> MissionTargetPick {
        self.target
    }

    #[must_use]
    pub fn agents(&self) -> &[MissionMember] {
        &self.agents
    }

    #[must_use]
    pub fn decoys(&self) -> &[MissionMember] {
        &self.decoys
    }

    #[must_use]
    pub fn page(&self) -> MissionDialogPage {
        self.page
    }

    /// Whether the mission-kind drop-down is open.
    #[must_use]
    pub fn list_open(&self) -> bool {
        self.list_open
    }

    /// Show a page (`FUN_0046c8a0`); the drop-down belongs to the first page
    /// and closes.
    pub fn show_page(&mut self, page: MissionDialogPage) {
        self.page = page;
        self.list_open = false;
    }

    /// Choose a listed kind (`FUN_0060c970`); an index past the list is
    /// ignored.
    pub fn select_kind(&mut self, index: usize) {
        if index < self.kinds.len() {
            self.kind = index;
        }
    }

    /// Flip a member's selection in whichever list holds it.
    pub fn toggle(&mut self, member: MissionMember) {
        if let Some(index) = self.selected.iter().position(|&m| m == member) {
            self.selected.remove(index);
        } else if self.agents.contains(&member) || self.decoys.contains(&member) {
            self.selected.push(member);
        }
    }

    /// Cases `0xca` (to the decoys) and `0xcb` (to the agents) of
    /// `FUN_0046c3c0`: the selected members of the source list join the end
    /// of the other list in their order, and every selection clears.
    pub fn move_selected(&mut self, to_decoys: bool) {
        let (from, to) = if to_decoys {
            (&mut self.agents, &mut self.decoys)
        } else {
            (&mut self.decoys, &mut self.agents)
        };
        let moved: Vec<MissionMember> = from
            .iter()
            .copied()
            .filter(|member| self.selected.contains(member))
            .collect();
        from.retain(|member| !moved.contains(member));
        to.extend(moved);
        self.selected.clear();
        self.agents_row = self.agents_row.min(self.agents.len().saturating_sub(1));
        self.decoys_row = self.decoys_row.min(self.decoys.len().saturating_sub(1));
    }
}

/// The box's key for a kind (`FUN_0046cab0`, switching on the MISSNSD
/// record's family at `+0x2c` and, for research `0x53`, its index at
/// `+0x18`). Other families take a rising global counter
/// (`DAT_006a8794`, `.data` initial value 100), so they sort after these
/// in the order they are added.
fn box_key(kind: MissionKind) -> u32 {
    match kind {
        MissionKind::Recruitment => 1,            // 0x55
        MissionKind::JediTraining => 2,           // 0x58
        MissionKind::SubdueUprising => 3,         // 0x57
        MissionKind::ShipDesignResearch => 4,     // 0x53, 0x20
        MissionKind::FacilityDesignResearch => 5, // 0x53, 0x21
        MissionKind::TroopTrainingResearch => 6,  // 0x53, 0x22
        MissionKind::Diplomacy => 7,              // 0x51
        MissionKind::InciteUprising => 8,         // 0x56
        MissionKind::Espionage => 9,              // 0x52
        MissionKind::Reconnaissance => 10,        // 0x54
        MissionKind::Assassination => 11,         // 0x63
        MissionKind::Abduction => 12,             // 0x62
        MissionKind::Rescue => 13,                // 0x61
        MissionKind::DeathStarSabotage => 14,     // 0x6a
        MissionKind::Sabotage => 15,              // 0x69
        _ => u32::MAX,
    }
}

/// The box's order for the kinds `available_kinds` gives in MISSNSD order:
/// `FUN_0046a9c0` builds each item with `FUN_0060ba00(FUN_0046cab0(record))`
/// (the key at item `+0xc`) and `FUN_005f59f0` inserts it into a list made
/// by `FUN_0060a790(1)` (vtable `0x0066e148`), whose compare
/// (`Class_0066e148_vt4`, mode 1) orders ascending by `+0xc`; an item goes
/// after its equals. The box shows the first item. A0
/// 2026-11-13-mission-dialogs/30..32 (Dodonna on Drall: Diplomacy,
/// Espionage) and 40..43 (Wedge on Drall: Ship Design Research, Diplomacy,
/// Espionage).
#[must_use]
pub fn box_order(mut kinds: Vec<MissionKind>) -> Vec<MissionKind> {
    kinds.sort_by_key(|kind| box_key(*kind));
    kinds
}

/// Whether a mission dialog is open.
#[derive(Debug, Clone, Default)]
pub struct MissionDialogState {
    dialog: Option<MissionDialog>,
}

impl MissionDialogState {
    /// Open the dialog for an order's team and target with the kinds the
    /// team may undertake. With no kind the order is dropped and nothing
    /// opens (`FUN_0042a320` -> `FUN_0041ce20(order, 0)`). The team starts as
    /// the agents and the decoys empty, as a fresh order `0x240` leaves them.
    /// The box lists `kinds` in the order given ([`box_order`]).
    pub fn open(
        &mut self,
        faction: MissionFaction,
        target: impl Into<MissionTargetPick>,
        team: Vec<MissionMember>,
        kinds: Vec<MissionKind>,
    ) -> bool {
        let target = target.into();
        if kinds.is_empty() || team.is_empty() {
            return false;
        }
        self.dialog = Some(MissionDialog {
            faction,
            target,
            kinds,
            kind: 0,
            page: MissionDialogPage::Mission,
            list_open: false,
            list_scroll: 0.0,
            list_hold: None,
            agents: team,
            decoys: Vec::new(),
            selected: Vec::new(),
            agents_row: 0,
            decoys_row: 0,
        });
        true
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.dialog.is_some()
    }

    #[must_use]
    pub fn dialog(&self) -> Option<&MissionDialog> {
        self.dialog.as_ref()
    }

    pub fn dialog_mut(&mut self) -> Option<&mut MissionDialog> {
        self.dialog.as_mut()
    }

    pub fn close(&mut self) {
        self.dialog = None;
    }

    /// Whether `point` falls on the open dialog, so the galaxy map under it
    /// takes no input. The dialog is a child window (style `0x46000000`),
    /// not modal.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.dialog.is_some() && rect_contains(dialog_rect(layout), egui::pos2(point.0, point.1))
    }

    /// "Begin Mission" (`0x66`): the order's mission becomes the selected
    /// kind and the order is submitted; the dialog closes whether or not the
    /// validator accepts it.
    pub fn begin(&mut self) -> Option<MissionDialogAction> {
        let dialog = self.dialog.take()?;
        Some(MissionDialogAction::Begin {
            kind: dialog.kind(),
            faction: dialog.faction,
            team: dialog.agents,
            decoys: dialog.decoys,
            target: dialog.target,
        })
    }
}

/// What the dialog asks of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionDialogAction {
    Begin {
        kind: MissionKind,
        faction: MissionFaction,
        team: Vec<MissionMember>,
        decoys: Vec<MissionMember>,
        target: MissionTargetPick,
    },
    /// "Encylopedia" (`0x67` -> `FUN_00429f30`, window `0x19`). port: it opens
    /// the Encyclopedia; the entry for the selected kind is not bound yet
    /// (`encyclopedia-window.md`).
    Encyclopedia,
}

/// An object target's portrait: `FUN_0042c3b0(.., 1, 1)` draws GOKRES
/// `(class +0x30 & 0xfff)`, the mini's id less `0x4000`
/// (`fleet-window.md`, "En route marks"), for a character (family
/// `0x30..0x3b`) and for a capital ship or the Death Star (no family the
/// function treats apart, so its last branch, `+0x30 & 0xfff` for a
/// portrait and `+ 0x4000` for a mini). A0 2026-11-16-edited-saves/x-46
/// (eb45cb97): the Death Star target shows GOKRES 1928 (its mini 18312 less
/// `0x4000`) in the box. `None` for any other target.
fn target_portrait(world: &GameWorld, target: MissionTargetPick) -> Option<u32> {
    let mini = if let Some(character) = target.character {
        character_picture(world.characters.get(character)?)?
    } else {
        let class = match target.object? {
            MissionTarget::CapitalShip { class, .. } => class,
            MissionTarget::DeathStar(fleet) => world
                .fleets
                .get(fleet)?
                .capital_ships
                .iter()
                .filter(|ship| ship.alive)
                .map(|ship| ship.class)
                .find(|class| {
                    world
                        .capital_ship_classes
                        .get(*class)
                        .is_some_and(rebellion_core::world::CapitalShipClass::is_death_star)
                })?,
            _ => return None,
        };
        crate::minis::capital_ship_mini_id(world.capital_ship_classes.get(class)?.dat_id)?
    };
    mini.checked_sub(0x4000)
}

/// The name of an object target, `None` for a system target.
fn object_target_name(world: &GameWorld, target: MissionTargetPick) -> Option<String> {
    use crate::system_window::{defense_facility_mini, special_force_mini, troop_mini};
    if let Some(character) = target.character {
        return world.characters.get(character).map(|c| c.name.clone());
    }
    let label = match target.object? {
        MissionTarget::SpecialForce(key) => {
            special_force_mini(world.special_forces.get(key)?.class_dat_id)?.1
        }
        MissionTarget::Troop(key) => troop_mini(world.troops.get(key)?.class_dat_id)?.1,
        MissionTarget::DefenseFacility(key) => {
            defense_facility_mini(world.defense_facilities.get(key)?.class_dat_id)?.1
        }
        // A facility reads as its class (`FUN_00437880`), as the System
        // window's product names do.
        MissionTarget::ManufacturingFacility(key) => {
            return facility_name(world, world.manufacturing_facilities.get(key)?.class_dat_id);
        }
        MissionTarget::ProductionFacility(key) => {
            return facility_name(world, world.production_facilities.get(key)?.class_dat_id);
        }
        MissionTarget::CapitalShip { class, .. } => {
            return world
                .capital_ship_classes
                .get(class)
                .map(|class| class.name.clone());
        }
        MissionTarget::Fighter { class, .. } => {
            return world
                .fighter_classes
                .get(class)
                .map(|class| class.name.clone());
        }
        MissionTarget::DeathStar(fleet) => {
            return world
                .fleets
                .get(fleet)?
                .capital_ships
                .iter()
                .filter(|ship| ship.alive)
                .filter_map(|ship| world.capital_ship_classes.get(ship.class))
                .find(|class| class.is_death_star())
                .map(|class| class.name.clone());
        }
    };
    Some(label.to_owned())
}

/// A facility class's name: its TEXTSTRA name, else its mini's label.
fn facility_name(world: &GameWorld, class: rebellion_core::ids::DatId) -> Option<String> {
    world
        .buildable_classes
        .get(&class)
        .map(|value| value.name.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            crate::manufacturing_window::class_mini(class).map(|(_, label)| label.to_owned())
        })
}

/// A kind's MISSNSD TEXTSTRA id: its name, and the source of its GOKRES icon.
const fn kind_text_id(kind: MissionKind) -> u16 {
    match kind {
        MissionKind::Diplomacy => 11280,
        MissionKind::Rescue => 11281,
        MissionKind::Sabotage => 11282,
        MissionKind::Espionage => 11283,
        MissionKind::Recruitment => 11286,
        MissionKind::Abduction => 11287,
        MissionKind::InciteUprising => 11328,
        MissionKind::DeathStarSabotage => 11329,
        MissionKind::SubdueUprising => 11392,
        MissionKind::Assassination => 11393,
        MissionKind::Reconnaissance => 11285,
        MissionKind::ShipDesignResearch => 11296,
        MissionKind::FacilityDesignResearch => 11297,
        MissionKind::TroopTrainingResearch => 11298,
        MissionKind::Move => 11265,
        MissionKind::Return => 11266,
        MissionKind::Autorouting => 11267,
        MissionKind::Adrift => 11268,
        // MISSNSD 0x58000042's TEXTSTRA id.
        MissionKind::JediTraining => 11330,
        // MISSNSD 0x71000043's TEXTSTRA id.
        MissionKind::Dagobah => 11331,
        MissionKind::Bounty => 11395,
        MissionKind::Palace => 11332,
        MissionKind::Pickup => 11394,
        MissionKind::Autoscrap => 0,
    }
}

/// A kind's TEXTSTRA name.
#[must_use]
pub const fn kind_name(kind: MissionKind) -> &'static str {
    match kind {
        MissionKind::Diplomacy => "Diplomacy",
        MissionKind::Rescue => "Rescue",
        MissionKind::Sabotage => "Sabotage",
        MissionKind::Espionage => "Espionage",
        MissionKind::Recruitment => "Recruitment",
        MissionKind::Abduction => "Abduction",
        MissionKind::InciteUprising => "Incite Uprising",
        MissionKind::DeathStarSabotage => "Death Star Sabotage",
        MissionKind::SubdueUprising => "Subdue Uprising",
        MissionKind::Assassination => "Assassination",
        MissionKind::Reconnaissance => "Reconnaissance",
        MissionKind::ShipDesignResearch => "Ship Design Research",
        MissionKind::FacilityDesignResearch => "Facility Design Research",
        MissionKind::TroopTrainingResearch => "Troop Training Research",
        MissionKind::Move => "Move",
        MissionKind::Return => "Return",
        MissionKind::Autorouting => "Autorouting",
        MissionKind::Adrift => "Adrift",
        MissionKind::JediTraining => "Jedi Training",
        MissionKind::Dagobah => "Dagobah",
        MissionKind::Bounty => "Bounty",
        MissionKind::Palace => "Palace",
        MissionKind::Pickup => "Pickup",
        MissionKind::Autoscrap => "",
    }
}

/// A kind's GOKRES icon: its TEXTSTRA id's low 12 bits, plus `0x1000` for the
/// Empire (`FUN_0046a9c0`, record `+0x30`).
#[must_use]
pub fn kind_icon(kind: MissionKind, faction: MissionFaction) -> u32 {
    let base = u32::from(kind_text_id(kind) & 0xfff);
    match faction {
        MissionFaction::Alliance => base,
        MissionFaction::Empire => base + 0x1000,
    }
}

/// A member's list image and name: the GOKRES mini `FUN_0042c3b0(.., 0, 1)`
/// draws over its layers (`member_layers`).
fn member_item(world: &GameWorld, member: MissionMember) -> Option<(Option<u32>, String)> {
    match member {
        MissionMember::Character(key) => {
            let character = world.characters.get(key)?;
            Some((character_picture(character), character.name.clone()))
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let (resource, name) = special_force_mini(unit.class_dat_id)?;
            Some((Some(resource), name.to_string()))
        }
    }
}

/// What `FUN_0042c3b0(.., 0, 1)` draws under a member's mini: the
/// personnel backdrop STRATEGY 11500, and 11502 while a character is
/// injured (`FUN_004f7800`). port: the en route, unbuilt and captive marks
/// are not drawn here.
fn member_layers(world: &GameWorld, member: MissionMember, mini: u32) -> Vec<(DllSource, u32)> {
    use crate::fleet_window::{mini_marks, MiniObject};
    let (object, injured) = match member {
        MissionMember::Character(key) => (
            MiniObject::Character,
            world
                .characters
                .get(key)
                .is_some_and(|character| character.injury > 0),
        ),
        MissionMember::SpecialForce(key) => (
            world
                .special_forces
                .get(key)
                .map_or(MiniObject::Character, |force| {
                    MiniObject::SpecialForce(force.class_dat_id)
                }),
            false,
        ),
    };
    mini_marks(object, mini, false, injured)
}

/// Where a bitmap of `size` pixels lands when centered in `bounds` at
/// `scale`: the original's `(box - size) / 2` offsets (`FUN_0046a9c0`), in
/// its own pixels, C's division truncating toward zero.
fn centered(bounds: egui::Rect, size: egui::Vec2, scale: f32) -> egui::Rect {
    let offset = |extent: f32, of: f32| ((extent / scale - of) / 2.0).trunc() * scale;
    egui::Rect::from_min_size(
        egui::pos2(
            bounds.min.x + offset(bounds.width(), size.x),
            bounds.min.y + offset(bounds.height(), size.y),
        ),
        size * scale,
    )
}

/// One wheel step through `len` items from `row`: down (a negative delta)
/// goes one on, up goes one back, and neither leaves the list.
pub(crate) fn scrolled(row: usize, len: usize, wheel: f32) -> usize {
    if wheel < 0.0 && row + 1 < len {
        row + 1
    } else if wheel > 0.0 {
        row.saturating_sub(1)
    } else {
        row
    }
}

/// Paint a bitmap at its own size scaled with the dialog, centered in `bounds`.
pub(crate) fn paint_centered(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    source: DllSource,
    resource_id: u32,
    bounds: egui::Rect,
    scale: f32,
) -> Option<egui::Rect> {
    let (texture_id, size) = cache
        .get(ctx, source, resource_id)
        .map(|texture| (texture.id(), texture.size_vec2()))?;
    let rect = centered(bounds, size, scale);
    painter.image(
        texture_id,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    Some(rect)
}

/// `FUN_005fc140` blits a bitmap at its own size (a zero width or height
/// means the bitmap's) and the control's window clips the rest, so a bitmap
/// wider than its control shows its left part.
fn native(rect: egui::Rect, size: egui::Vec2, scale: f32) -> egui::Rect {
    egui::Rect::from_min_size(rect.min, size * scale)
}

pub(crate) fn paint(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    resource_id: u32,
    rect: egui::Rect,
    scale: f32,
) {
    #[cfg(test)]
    crate::fleet_window::tests::PAINTED.with(|painted| {
        painted.borrow_mut().push((resource_id, rect.min));
    });
    if let Some(texture) = cache.get(ctx, DllSource::Strategy, resource_id) {
        painter
            .with_clip_rect(painter.clip_rect().intersect(rect))
            .image(
                texture.id(),
                native(rect, texture.size_vec2(), scale),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
    }
}

/// A two-state bitmap button (`FUN_00602150`): the second bitmap shows while
/// it is held or, for a tab, while it is the current page.
pub(crate) fn button(
    ui: &egui::Ui,
    cache: &mut BmpCache,
    rect: egui::Rect,
    id: impl std::hash::Hash,
    (normal, pressed): (u32, u32),
    down: bool,
    scale: f32,
) -> bool {
    let response = ui.interact(rect, ui.id().with(id), egui::Sense::click());
    let (pointer, primary_down) = ui.ctx().input(|input| {
        (
            input.pointer.interact_pos(),
            input.pointer.button_down(egui::PointerButton::Primary),
        )
    });
    let held = primary_down && pointer.is_some_and(|point| rect_contains(rect, point));
    paint(
        ui.painter(),
        ui.ctx(),
        cache,
        if down || held { pressed } else { normal },
        rect,
        scale,
    );
    exact_clicked(&response, rect)
}

/// The dialog's screen rectangle.
fn dialog_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, MISSION_DIALOG_WIDTH, MISSION_DIALOG_HEIGHT)
}

/// A `width` by `height` window of the galaxy view, placed as
/// `FUN_00606980` places it in the galaxy view's rectangle (parent
/// `+0xcc..+0xd8`): `left + (right - left - width) / 2`, the division
/// truncating, in the 640 by 480 canvas (A0 2026-10-29: the Objectives
/// window at (169, 81)); on whole screen pixels so its bitmaps are not
/// resampled.
pub(crate) fn galaxy_centered_rect(layout: CockpitLayout, width: f32, height: f32) -> egui::Rect {
    let scale = layout.scale;
    let size = egui::vec2(width * scale, height * scale);
    if scale <= 0.0 {
        return egui::Rect::from_min_size(egui::pos2(layout.galaxy.x, layout.galaxy.y), size);
    }
    let place = |start: f32, extent: f32, origin: f32, own: f32| {
        let start = ((start - origin) / scale).round();
        let extent = (extent / scale).round();
        (origin + (start + ((extent - own) / 2.0).trunc()) * scale).round()
    };
    egui::Rect::from_min_size(
        egui::pos2(
            place(layout.galaxy.x, layout.galaxy.width, layout.canvas.x, width),
            place(layout.galaxy.y, layout.galaxy.height, layout.canvas.y, height),
        ),
        size,
    )
}

/// Draw the open dialog, if any, and report what the player asked for.
#[expect(
    clippy::too_many_lines,
    reason = "The dialog paints in the original's control order; splitting it hides that order."
)]
pub fn draw_mission_dialog(
    ctx: &egui::Context,
    world: &GameWorld,
    state: &mut MissionDialogState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<MissionDialogAction> {
    let dialog = state.dialog.as_mut()?;
    let scale = layout.scale;
    let side = usize::from(dialog.faction == MissionFaction::Empire);
    let window = dialog_rect(layout);
    let mut action = None;
    let mut close = false;
    let mut begin = false;

    // Above the modeless windows, which share Foreground and raise the
    // focused one each frame (`system_window.rs`).
    egui::Area::new(egui::Id::new("original-mission-dialog"))
        .fixed_pos(window.min)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(window.size(), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(frame);
            let at = |x: f32, y: f32, w: f32, h: f32| logical_rect(frame, scale, x, y, w, h);

            // The page panel, then the title strip: the 240-pixel bar laid
            // twice, 30 pixels apart (a 270 by 17 surface), of which
            // FUN_005fcc30(.., 2, 2, 0, 0, 0xff, 0x10) copies 255 by 16 to
            // (2, 2): A0 menus2/11 keeps the panel under the bar's last row
            // and right of x 256.
            let panel = match dialog.page {
                MissionDialogPage::Mission => PANEL_MISSION,
                MissionDialogPage::Agents => PANEL_AGENTS,
            };
            paint(&painter, ctx, cache, panel, frame, scale);
            let title = [TITLE_ALLIANCE, TITLE_EMPIRE][side];
            let strip =
                painter.with_clip_rect(painter.clip_rect().intersect(at(2.0, 2.0, 255.0, 16.0)));
            for x in [2.0, 32.0] {
                paint(&strip, ctx, cache, title, at(x, 2.0, 240.0, 17.0), scale);
            }
            // FUN_0046a9c0 draws the first page's title black and leaves the
            // white of "Target" set for the second page's.
            let title_color = match dialog.page {
                MissionDialogPage::Mission => egui::Color32::BLACK,
                MissionDialogPage::Agents => egui::Color32::WHITE,
            };
            crate::theme::paint_gdi_text(
                &painter,
                at(5.0, 2.0, 240.0, 16.0),
                scale,
                TEXT_FONT,
                CREATE_MISSION,
                title_color,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );

            if button(
                ui,
                cache,
                at(242.0, 3.0, 14.0, 14.0),
                "close",
                CLOSE,
                false,
                scale,
            ) {
                close = true;
            }
            for (page, x, art) in [
                (MissionDialogPage::Mission, 0.0, MISSION_TAB[side]),
                (MissionDialogPage::Agents, 130.0, AGENTS_TAB[side]),
            ] {
                let rect = at(7.0 + x, 20.0, 116.0, 33.0);
                if button(
                    ui,
                    cache,
                    rect,
                    ("tab", x as i32),
                    art,
                    dialog.page == page,
                    scale,
                ) {
                    dialog.show_page(page);
                }
            }

            match dialog.page {
                MissionDialogPage::Mission => {
                    crate::theme::paint_gdi_text(
                        &painter,
                        at(37.0, 195.0, 185.0, 16.0),
                        scale,
                        TEXT_FONT,
                        TARGET,
                        egui::Color32::WHITE,
                        (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                    );
                    let kind = dialog.kind();
                    let (x, y, w, h) = KIND_BOX;
                    paint_kind(
                        &painter,
                        ctx,
                        cache,
                        kind,
                        dialog.faction,
                        at(x, y, w, h),
                        scale,
                        KIND_TEXT,
                    );
                    // An object target shows its name over its picture: a
                    // character or a capital ship its portrait
                    // (FUN_0042c3b0(.., 1, 1); A0 2026-11-15-partial-cells/
                    // rescue-dialog-han-on-leia-fleet90-d1428, 2026-11-16-
                    // edited-saves/x-46). port: other objects' pictures are
                    // not drawn.
                    if let Some(name) = object_target_name(world, dialog.target) {
                        if let Some(portrait) = target_portrait(world, dialog.target) {
                            // The 80-high portrait is clipped to the 79-high
                            // box control (A0 kinds2/50..55: the row under
                            // the box keeps the dialog's ground).
                            let (x, y, w, h) = TARGET_BOX;
                            let boxed = painter.with_clip_rect(at(x, y, w, h));
                            paint_centered(
                                &boxed,
                                ctx,
                                cache,
                                DllSource::Gokres,
                                portrait,
                                at(x, y, w, h),
                                scale,
                            );
                        }
                        let (x, y, w) = TARGET_NAME;
                        crate::theme::paint_gdi_text(
                            &painter,
                            at(x, y, w, 16.0),
                            scale,
                            TEXT_FONT,
                            &name,
                            egui::Color32::WHITE,
                            (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
                        );
                    } else if let Some(system) = world.systems.get(dialog.target.system) {
                        let (x, y, w, h) = TARGET_BOX;
                        paint_centered(
                            &painter,
                            ctx,
                            cache,
                            DllSource::Strategy,
                            planet_resource_id(system.dat_id),
                            at(x, y, w, h),
                            scale,
                        );
                        let (x, y, w) = TARGET_NAME;
                        crate::theme::paint_gdi_text(
                            &painter,
                            at(x, y, w, 16.0),
                            scale,
                            TEXT_FONT,
                            &system.name,
                            egui::Color32::WHITE,
                            (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
                        );
                    }
                    // FUN_0046a9c0: the mission box (+0x128) TEXTSTRA
                    // 0x8520, its Missions button 0x8510.
                    let (x, y, w, h) = KIND_BOX;
                    crate::win_tooltip::window_tool(ui, at(x, y, w, h), "kind", 0x8520, true);
                    crate::win_tooltip::window_tool(
                        ui,
                        at(101.0, 174.0, 65.0, 18.0),
                        "missions",
                        0x8510,
                        true,
                    );
                    if button(
                        ui,
                        cache,
                        at(101.0, 174.0, 65.0, 18.0),
                        "missions",
                        MISSIONS,
                        false,
                        scale,
                    ) {
                        dialog.list_open = !dialog.list_open;
                        // hyp: the list opens with the shown kind at its top.
                        dialog.list_scroll = dialog.kind as f32 * KIND_BOX.3;
                        dialog.list_hold = None;
                    }
                }
                MissionDialogPage::Agents => {
                    paint(
                        &painter,
                        ctx,
                        cache,
                        AGENTS_HEADER[side],
                        at(8.0, 65.0, 108.0, 27.0),
                        scale,
                    );
                    paint(
                        &painter,
                        ctx,
                        cache,
                        DECOYS_HEADER[side],
                        at(136.0, 65.0, 108.0, 27.0),
                        scale,
                    );
                    for to_decoys in [false, true] {
                        let (x, y) = if to_decoys { DECOYS_LIST } else { AGENTS_LIST };
                        let list = at(x, y, LIST_SIZE.0, LIST_SIZE.1);
                        let (members, row) = if to_decoys {
                            (dialog.decoys.clone(), &mut dialog.decoys_row)
                        } else {
                            (dialog.agents.clone(), &mut dialog.agents_row)
                        };
                        let result = draw_list(
                            ui,
                            &painter,
                            cache,
                            world,
                            &members,
                            &dialog.selected,
                            row,
                            list,
                            scale,
                            to_decoys,
                            ITEM_FONT,
                        );
                        if let Some(member) = result.clicked {
                            dialog.toggle(member);
                        }
                        // A double click (notification 0x309) acts as the
                        // move out of its list (FUN_0046c3c0.c:80-86).
                        if let Some(member) = result.double_clicked {
                            dialog.selected.push(member);
                            dialog.move_selected(!to_decoys);
                        }
                    }
                    // 0xca / 0xcb: TEXTSTRA 0x8505, 0x8506.
                    for (y, id, string) in
                        [(136.0, "to-decoys", 0x8505), (221.0, "to-agents", 0x8506)]
                    {
                        crate::win_tooltip::window_tool(
                            ui,
                            at(120.0, y, 16.0, 16.0),
                            id,
                            string,
                            true,
                        );
                    }
                    if button(
                        ui,
                        cache,
                        at(120.0, 136.0, 16.0, 16.0),
                        "to-decoys",
                        TO_DECOYS,
                        false,
                        scale,
                    ) {
                        dialog.move_selected(true);
                    }
                    if button(
                        ui,
                        cache,
                        at(120.0, 221.0, 16.0, 16.0),
                        "to-agents",
                        TO_AGENTS,
                        false,
                        scale,
                    ) {
                        dialog.move_selected(false);
                    }
                }
            }

            // Cancel 0x8503, Begin Mission 0x8502, "Encylopedia" 0x8031; the
            // close box has none.
            for (x, id, string) in [
                (170.0, "cancel", 0x8503),
                (102.0, "begin", 0x8502),
                (33.0, "encyclopedia", 0x8031),
            ] {
                crate::win_tooltip::window_tool(ui, at(x, 320.0, 64.0, 33.0), id, string, true);
            }
            if button(
                ui,
                cache,
                at(170.0, 320.0, 64.0, 33.0),
                "cancel",
                CANCEL,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(102.0, 320.0, 64.0, 33.0),
                "begin",
                BEGIN,
                false,
                scale,
            ) {
                begin = true;
            }
            if button(
                ui,
                cache,
                at(33.0, 320.0, 64.0, 33.0),
                "encyclopedia",
                ENCYCLOPEDIA,
                false,
                scale,
            ) {
                action = Some(MissionDialogAction::Encyclopedia);
            }

            // The drop-down (FUN_0060cac0, selection_list): 113-pixel items,
            // one visible, under the 200-wide box. The shown kind's name is
            // white and the others' gray (the item's +0x60; A0 kinds/31).
            if dialog.list_open && dialog.page == MissionDialogPage::Mission {
                let (x, y, w, h) = KIND_BOX;
                let drop = crate::selection_list::DropDown {
                    x,
                    top: y + h,
                    width: w,
                    item_height: h,
                    visible: 1,
                    black_edge: false,
                };
                let kinds = dialog.kinds.clone();
                let (shown, faction) = (dialog.kind, dialog.faction);
                let mut scroll = dialog.list_scroll;
                let mut hold = dialog.list_hold;
                let outcome = crate::selection_list::draw(
                    ui,
                    &painter,
                    cache,
                    frame,
                    scale,
                    ui.id().with("kind-list"),
                    drop,
                    kinds.len(),
                    &mut scroll,
                    &mut hold,
                    |items, cache, index, rect| {
                        let colour = if index == shown {
                            KIND_TEXT
                        } else {
                            LIST_KIND_TEXT
                        };
                        paint_kind(items, ctx, cache, kinds[index], faction, rect, scale, colour);
                    },
                );
                dialog.list_scroll = scroll;
                dialog.list_hold = hold;
                if let Some(row) = outcome.chosen {
                    dialog.select_kind(row);
                }
                if outcome.closed {
                    dialog.list_open = false;
                }
            }
        });

    if begin {
        return state.begin();
    }
    if close {
        state.close();
    }
    action
}

/// One mission kind item: the 130 by 65 GOKRES icon centered in the box, and
/// its name centered along the box's top edge, as A0 menus2/11-13 show it.
/// port: placed from the captures; `FUN_0060c210`'s rectangle comes from
/// the item's `+0x30`/`+0x34`, not traced.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit painter, cache and geometry inputs at this drawing boundary."
)]
fn paint_kind(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    kind: MissionKind,
    faction: MissionFaction,
    bounds: egui::Rect,
    scale: f32,
    colour: egui::Color32,
) {
    let _ = paint_centered(
        painter,
        ctx,
        cache,
        DllSource::Gokres,
        kind_icon(kind, faction),
        bounds,
        scale,
    );
    crate::theme::paint_gdi_text(
        painter,
        bounds,
        scale,
        crate::DEFAULT_FONT,
        kind_name(kind),
        colour,
        (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
    );
}

#[derive(Default)]
struct ListResult {
    clicked: Option<MissionMember>,
    double_clicked: Option<MissionMember>,
}

/// An agents or decoys list (`CoolDragList`, `FUN_00607ea0`): 107 by 59
/// items, each a centered mini with its name under it. port: its scroll bar
/// (base 0x29fc) is not drawn; the wheel scrolls it by one item.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit painter, cache and geometry inputs at this drawing boundary."
)]
fn draw_list(
    ui: &egui::Ui,
    painter: &egui::Painter,
    cache: &mut BmpCache,
    world: &GameWorld,
    members: &[MissionMember],
    selected: &[MissionMember],
    row: &mut usize,
    list: egui::Rect,
    scale: f32,
    decoys: bool,
    font: u16,
) -> ListResult {
    let mut result = ListResult::default();
    let list_painter = painter.with_clip_rect(list);
    // The items sit over the list, so the pointer's position, not the list's
    // hover, decides where a wheel notch goes.
    let over = ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|point| rect_contains(list, point));
    if over {
        let wheel = ui.ctx().input(|input| input.raw_scroll_delta.y);
        *row = scrolled(*row, members.len(), wheel);
    }
    for (index, &member) in members.iter().enumerate().skip(*row) {
        let offset = (index - *row) as f32 * ITEM_SIZE.1;
        if offset >= LIST_SIZE.1 {
            break;
        }
        let item = logical_rect(list, scale, 0.0, offset, ITEM_SIZE.0, ITEM_SIZE.1);
        let Some((mini, name)) = member_item(world, member) else {
            continue;
        };
        let image = mini.and_then(|id| {
            for (source, layer) in member_layers(world, member, id) {
                paint_centered(&list_painter, ui.ctx(), cache, source, layer, item, scale);
            }
            paint_centered(
                &list_painter,
                ui.ctx(),
                cache,
                DllSource::Gokres,
                id,
                item,
                scale,
            )
        });
        let color = if selected.contains(&member) {
            ITEM_TEXT_SELECTED
        } else {
            ITEM_TEXT
        };
        let top = image.map_or(item.center().y, |rect| rect.max.y);
        let (ascent, descent) = crate::theme::original_font_metrics(font);
        for (line, text) in item_name_lines(&name).iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "A line or two.")]
            let line_top = top + line as f32 * (ascent + descent) * scale;
            crate::theme::paint_gdi_text(
                &list_painter,
                egui::Rect::from_min_max(
                    egui::pos2(item.min.x, line_top),
                    egui::pos2(item.max.x, item.max.y),
                ),
                scale,
                font,
                text,
                color,
                (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
            );
        }
        let clip = item.intersect(list);
        let response = ui.interact(
            clip,
            ui.id().with(("item", decoys, index)),
            egui::Sense::click(),
        );
        if response.double_clicked() {
            result.double_clicked = Some(member);
        } else if exact_clicked(&response, clip) {
            result.clicked = Some(member);
        }
    }
    result
}

#[cfg(test)]
#[path = "mission_dialog_tests.rs"]
mod tests;
