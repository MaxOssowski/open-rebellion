//! The object pop-up menu a right-click on a character, special force or
//! fleet opens in a system window (`FUN_004ac5c0`). Recovery notes:
//! `ghidra/notes/object-popup-menu.md` and `ghidra/notes/move-order.md`.
//!
//! `FUN_0051d990` lists the orders the selection's class offers, sorts them
//! by their STRATEGY `RT_RCDATA` record's key, and always adds Encyclopedia
//! and Status. Each record names its text (word 5, TEXTSTRA), its parent
//! submenu (word 1) and its sort key (word 2).

use egui_macroquad::egui;
use rebellion_core::ids::{CharacterKey, FleetKey, SpecialForceKey};
use rebellion_core::missions::MissionMember;

use crate::bmp_cache::BmpCache;
use crate::cockpit::{CockpitFaction, CockpitLayout, CockpitViewport};
use crate::game_menu::{draw_game_menu, GameMenuEntry, GameMenuPlacement, GameMenuResponse};

/// What an item does when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMenuCommand {
    Move,
    ConfirmedMove,
    Mission,
    Command,
    Encyclopedia,
    Status,
    Retire,
    Bombardment,
    Assault,
    Rename,
    Scrap,
}

/// The object a menu opens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuObject {
    Character(CharacterKey),
    SpecialForce(SpecialForceKey),
    Fleet(FleetKey),
}

impl MenuObject {
    /// The object as a mission team member; a fleet is none.
    #[must_use]
    pub const fn mission_member(self) -> Option<MissionMember> {
        match self {
            Self::Character(key) => Some(MissionMember::Character(key)),
            Self::SpecialForce(key) => Some(MissionMember::SpecialForce(key)),
            Self::Fleet(_) => None,
        }
    }
}

/// One STRATEGY menu record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMenuItem {
    /// The order kind, which names the record.
    pub kind: u16,
    pub command: ObjectMenuCommand,
    /// Record word 2.
    pub sort_key: u16,
    /// Record word 5.
    pub label_string_id: u16,
    pub label: &'static str,
    /// Record word 4: the item opens a submenu.
    pub submenu: bool,
}

const fn item(
    kind: u16,
    command: ObjectMenuCommand,
    sort_key: u16,
    label_string_id: u16,
    label: &'static str,
    submenu: bool,
) -> ObjectMenuItem {
    ObjectMenuItem {
        kind,
        command,
        sort_key,
        label_string_id,
        label,
        submenu,
    }
}

const MOVE: ObjectMenuItem = item(0x201, ObjectMenuCommand::Move, 10, 12312, "Move", false);
const CONFIRMED_MOVE: ObjectMenuItem = item(
    0x202,
    ObjectMenuCommand::ConfirmedMove,
    12,
    12311,
    "Confirmed Move",
    false,
);
const MISSION: ObjectMenuItem = item(
    0x240,
    ObjectMenuCommand::Mission,
    300,
    12320,
    "Mission",
    false,
);
/// The parent of None, Admiral, General and Commander (`0x260..0x263`).
const COMMAND: ObjectMenuItem = item(
    0x160,
    ObjectMenuCommand::Command,
    400,
    12352,
    "Command",
    true,
);
const ENCYCLOPEDIA: ObjectMenuItem = item(
    0x100,
    ObjectMenuCommand::Encyclopedia,
    1000,
    12292,
    "Encyclopedia",
    false,
);
const STATUS: ObjectMenuItem = item(
    0x103,
    ObjectMenuCommand::Status,
    1001,
    12293,
    "Status",
    false,
);
const RETIRE: ObjectMenuItem = item(
    0x242,
    ObjectMenuCommand::Retire,
    2002,
    12336,
    "Retire",
    false,
);

/// The parent of the four bombardment targets (`0x220..0x223`).
const BOMBARDMENT: ObjectMenuItem = item(
    0x120,
    ObjectMenuCommand::Bombardment,
    200,
    12313,
    "Planetary Bombardment",
    true,
);
const ASSAULT: ObjectMenuItem = item(
    0x234,
    ObjectMenuCommand::Assault,
    220,
    12318,
    "Planetary Assault",
    false,
);
const RENAME: ObjectMenuItem = item(
    0x203,
    ObjectMenuCommand::Rename,
    500,
    12291,
    "Rename",
    false,
);
const SCRAP: ObjectMenuItem = item(0x200, ObjectMenuCommand::Scrap, 2000, 12295, "Scrap", false);

/// One row of an open object menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMenuRow {
    pub item: ObjectMenuItem,
    pub enabled: bool,
}

impl ObjectMenuRow {
    #[must_use]
    pub const fn entry(&self) -> GameMenuEntry<'static> {
        GameMenuEntry {
            label: self.item.label,
            icon: None,
            enabled: self.enabled,
            submenu: self.item.submenu,
        }
    }
}

/// The rows the menu shows for `selection`, in sort-key order.
///
/// A character's class offers Move, Confirmed Move, Retire and Mission
/// (`FUN_00536bc0`) and the Command ranks (`FUN_004f2400`); a special
/// force's only the first four (`FUN_00503fa0`). A fleet's class offers Move,
/// Confirmed Move, the bombardments, Assault, Rename and Scrap
/// (`FUN_004ff8e0`); the bombardment targets sit under their submenu parent.
/// Kinds `0x204`, `0x241` and `0x268` are offered too but have no STRATEGY
/// record, so they never show.
/// An empty selection lists only Encyclopedia and Status, both disabled.
///
/// - Mission is enabled when `mission_enabled` says so
///   (`MissionState::mission_order_enabled`). port: the global gate
///   `FUN_0051de80` is taken as clear.
/// - Encyclopedia is enabled for a single selection.
/// - port: Move, Confirmed Move, Command, Status, Retire and the fleet orders
///   stay disabled until their windows and orders are ported. In the original, Status is
///   enabled for a single selection that is not a system, and Command is a
///   submenu parent.
#[must_use]
pub fn object_menu_rows(
    selection: Option<MenuObject>,
    mission_enabled: bool,
) -> Vec<ObjectMenuRow> {
    let offered: &[ObjectMenuItem] = match selection {
        Some(MenuObject::Character(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION, COMMAND],
        Some(MenuObject::SpecialForce(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION],
        Some(MenuObject::Fleet(_)) => &[MOVE, CONFIRMED_MOVE, BOMBARDMENT, ASSAULT, RENAME, SCRAP],
        None => &[],
    };
    let mut rows: Vec<ObjectMenuRow> = offered
        .iter()
        .chain(&[ENCYCLOPEDIA, STATUS])
        .map(|&item| ObjectMenuRow {
            item,
            enabled: match item.command {
                ObjectMenuCommand::Mission => mission_enabled,
                ObjectMenuCommand::Encyclopedia => selection.is_some(),
                _ => false,
            },
        })
        .collect();
    rows.sort_by_key(|row| row.item.sort_key);
    rows
}

/// An open object pop-up menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMenuState {
    selection: Option<MenuObject>,
    rows: Vec<ObjectMenuRow>,
    /// The 640 by 480 canvas point of the right-button release.
    point: (f32, f32),
}

impl ObjectMenuState {
    /// Open the menu for `selection` at a canvas `point`.
    #[must_use]
    pub fn new(selection: Option<MenuObject>, mission_enabled: bool, point: (i16, i16)) -> Self {
        Self {
            selection,
            rows: object_menu_rows(selection, mission_enabled),
            point: (f32::from(point.0), f32::from(point.1)),
        }
    }

    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The row that issues `command`, counted from the top.
    #[must_use]
    pub fn row_of(&self, command: ObjectMenuCommand) -> Option<usize> {
        self.rows.iter().position(|row| row.item.command == command)
    }
}

const MENU_ID: &str = "original_object_menu";

/// The open menu's screen rectangle, once egui has laid it out.
#[must_use]
pub fn object_menu_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.memory(|memory| memory.area_rect(egui::Id::new(MENU_ID)))
}

/// The galaxy view in canvas coordinates: the menu's owner window
/// (`FUN_004ac5c0` maps the point into it and `FUN_00442380` opens there).
fn galaxy_owner(layout: CockpitLayout) -> CockpitViewport {
    let scale = layout.scale.max(f32::EPSILON);
    CockpitViewport {
        x: (layout.galaxy.x - layout.canvas.x) / scale,
        y: (layout.galaxy.y - layout.canvas.y) / scale,
        width: layout.galaxy.width / scale,
        height: layout.galaxy.height / scale,
    }
}

/// Draw the open menu. Returns the chosen command and the selection it acts
/// on; a choice, a press outside or Escape closes the menu.
pub fn draw_object_menu(
    ctx: &egui::Context,
    menu: &mut Option<ObjectMenuState>,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    input_enabled: bool,
) -> Option<(ObjectMenuCommand, Option<MenuObject>)> {
    let open = menu.as_ref()?;
    let entries: Vec<GameMenuEntry<'static>> = open.rows.iter().map(ObjectMenuRow::entry).collect();
    let placement = GameMenuPlacement {
        owner: galaxy_owner(layout),
        point: open.point,
    };
    match draw_game_menu(
        ctx,
        egui::Id::new(MENU_ID),
        cache,
        layout,
        faction,
        placement,
        &entries,
        input_enabled,
    ) {
        GameMenuResponse::Open => None,
        GameMenuResponse::Dismissed => {
            *menu = None;
            None
        }
        GameMenuResponse::Chosen(index) => {
            let chosen = (open.rows[index].item.command, open.selection);
            *menu = None;
            Some(chosen)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
        rows.iter().map(|row| row.item.label).collect()
    }

    fn enabled(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
        rows.iter()
            .filter(|row| row.enabled)
            .map(|row| row.item.label)
            .collect()
    }

    #[test]
    fn a_characters_menu_matches_the_manual_and_the_strategy_records() {
        // Manual p. 99 and Fig. 3.45; STRATEGY RT_RCDATA words 2 and 5;
        // FUN_004f2400 over FUN_00536bc0.
        let rows = object_menu_rows(Some(MenuObject::Character(CharacterKey::default())), true);
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Mission",
                "Command",
                "Encyclopedia",
                "Status",
                "Retire"
            ]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x201, 0x202, 0x240, 0x160, 0x100, 0x103, 0x242]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12312, 12311, 12320, 12352, 12292, 12293, 12336]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [10, 12, 300, 400, 1000, 1001, 2002]
        );
        // Record 0x160's word 4 marks the Command submenu parent.
        assert_eq!(
            rows.iter()
                .filter(|row| row.item.submenu)
                .map(|row| row.item.label)
                .collect::<Vec<_>>(),
            ["Command"]
        );
    }

    #[test]
    fn a_special_forces_menu_has_no_command_submenu() {
        // FUN_00503fa0 copies only the unit list of FUN_00536bc0.
        let rows = object_menu_rows(
            Some(MenuObject::SpecialForce(SpecialForceKey::default())),
            true,
        );
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Mission",
                "Encyclopedia",
                "Status",
                "Retire"
            ]
        );
    }

    #[test]
    fn a_fleets_menu_matches_the_manual_and_the_strategy_records() {
        // Manual Fig. 3.64; FUN_004ff8e0's list; STRATEGY RT_RCDATA words 2,
        // 4 and 5 (ghidra/notes/move-order.md, "The Fleet menu").
        let rows = object_menu_rows(Some(MenuObject::Fleet(FleetKey::default())), true);
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Planetary Bombardment",
                "Planetary Assault",
                "Rename",
                "Encyclopedia",
                "Status",
                "Scrap"
            ]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x201, 0x202, 0x120, 0x234, 0x203, 0x100, 0x103, 0x200]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12312, 12311, 12313, 12318, 12291, 12292, 12293, 12295]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [10, 12, 200, 220, 500, 1000, 1001, 2000]
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.item.submenu)
                .map(|row| row.item.label)
                .collect::<Vec<_>>(),
            ["Planetary Bombardment"]
        );
        // A fleet is no mission member, so Mission's gate never reaches it.
        assert_eq!(enabled(&rows), ["Encyclopedia"]);
    }

    #[test]
    fn only_characters_and_special_forces_are_mission_members() {
        let character = CharacterKey::default();
        let unit = SpecialForceKey::default();
        assert_eq!(
            MenuObject::Character(character).mission_member(),
            Some(MissionMember::Character(character))
        );
        assert_eq!(
            MenuObject::SpecialForce(unit).mission_member(),
            Some(MissionMember::SpecialForce(unit))
        );
        assert_eq!(
            MenuObject::Fleet(FleetKey::default()).mission_member(),
            None
        );
    }

    #[test]
    fn mission_follows_the_order_rule_and_encyclopedia_needs_one_selection() {
        // FUN_0051fe20 enables Mission; FUN_0051d990 enables Encyclopedia
        // for a single selection.
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, true)),
            ["Mission", "Encyclopedia"]
        );
        assert_eq!(
            enabled(&object_menu_rows(character, false)),
            ["Encyclopedia"]
        );
    }

    #[test]
    fn an_empty_selection_lists_encyclopedia_and_status_disabled() {
        // FUN_0051d990 always adds 0x100 and 0x103; neither is enabled
        // without exactly one selected object.
        let rows = object_menu_rows(None, true);
        assert_eq!(labels(&rows), ["Encyclopedia", "Status"]);
        assert!(enabled(&rows).is_empty());
    }

    #[test]
    fn a_row_becomes_a_menu_entry_without_an_icon() {
        // The character records carry no icon (words 7 and 8 are zero).
        let rows = object_menu_rows(Some(MenuObject::Character(CharacterKey::default())), true);
        let command = rows.iter().find(|row| row.item.label == "Command").unwrap();
        assert_eq!(
            command.entry(),
            GameMenuEntry {
                label: "Command",
                icon: None,
                enabled: false,
                submenu: true,
            }
        );
        let mission = rows.iter().find(|row| row.item.label == "Mission").unwrap();
        assert!(mission.entry().enabled && !mission.entry().submenu);
    }

    fn galaxy_layout() -> CockpitLayout {
        crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(1280.0, 960.0)
    }

    /// Draw `menu` for two layout frames, then click the centre of row
    /// `row`. Returns the choice and the menu's rectangle.
    fn click_row(
        menu: &mut Option<ObjectMenuState>,
        row: usize,
    ) -> (Option<(ObjectMenuCommand, Option<MenuObject>)>, egui::Rect) {
        let layout = galaxy_layout();
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let count = menu.as_ref().map_or(0, |open| open.rows.len());
        let mut chosen = None;
        let mut rect = egui::Rect::NOTHING;
        for frame in 0..4 {
            let mut events = Vec::new();
            if frame >= 2 {
                rect = object_menu_rect(&ctx).expect("the menu is laid out");
                let height = (rect.height() - 2.0 * layout.scale) / count as f32;
                let pos = egui::pos2(
                    rect.center().x,
                    rect.min.y + 2.0 * layout.scale + height * (row as f32 + 0.5),
                );
                events.push(egui::Event::PointerMoved(pos));
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 2,
                    modifiers: egui::Modifiers::default(),
                });
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 960.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                if let Some(choice) = draw_object_menu(
                    ctx,
                    menu,
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    true,
                ) {
                    chosen = Some(choice);
                }
            });
        }
        (chosen, rect)
    }

    #[test]
    fn a_characters_menu_lists_mission_third_of_seven_rows() {
        // STRATEGY RT_RCDATA sort keys: Move 10, Confirmed Move 12, Mission
        // 300, Command, Encyclopedia 1000, Status 1001, Retire 2002.
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let menu = ObjectMenuState::new(agent, true, (100, 100));
        assert_eq!(menu.row_count(), 7);
        assert_eq!(menu.row_of(ObjectMenuCommand::Mission), Some(2));
        assert_eq!(menu.row_of(ObjectMenuCommand::Encyclopedia), Some(4));

        // FUN_0051d990 lists only Encyclopedia and Status for no selection.
        let empty = ObjectMenuState::new(None, true, (100, 100));
        assert_eq!(empty.row_of(ObjectMenuCommand::Mission), None);
    }

    #[test]
    fn choosing_mission_returns_it_with_the_selection_and_closes_the_menu() {
        // FUN_004424c0 reports the kind to the owner's vtable +0x20
        // (FUN_004ac730), which acts on the copied selection +0x11c.
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut menu = Some(ObjectMenuState::new(agent, true, (100, 100)));
        let (chosen, _) = click_row(&mut menu, 2);
        assert_eq!(chosen, Some((ObjectMenuCommand::Mission, agent)));
        assert_eq!(menu, None);
    }

    #[test]
    fn a_disabled_row_keeps_the_menu_open() {
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut menu = Some(ObjectMenuState::new(agent, true, (100, 100)));
        let (chosen, _) = click_row(&mut menu, 0);
        assert_eq!(chosen, None);
        assert!(menu.is_some());
    }

    #[test]
    fn the_menu_opens_in_the_galaxy_view_and_flips_at_its_edges() {
        // FUN_00442860 flips at the owner's edges; the owner is the galaxy
        // view (55, 40) to (540, 390) for the Alliance (FUN_00421c70).
        let mut offset = galaxy_layout();
        offset.canvas.x += 10.0;
        offset.canvas.y += 20.0;
        offset.galaxy.x += 10.0;
        offset.galaxy.y += 20.0;
        assert_eq!(
            galaxy_owner(offset),
            CockpitViewport {
                x: 55.0,
                y: 40.0,
                width: 485.0,
                height: 350.0,
            }
        );
        let layout = galaxy_layout();
        assert_eq!(
            galaxy_owner(layout),
            CockpitViewport {
                x: 55.0,
                y: 40.0,
                width: 485.0,
                height: 350.0,
            }
        );
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut inside = Some(ObjectMenuState::new(agent, true, (100, 100)));
        let (_, rect) = click_row(&mut inside, 0);
        assert_eq!(
            rect.min,
            egui::pos2(
                layout.canvas.x + 100.0 * layout.scale,
                layout.canvas.y + 100.0 * layout.scale
            )
        );
        // Near the galaxy view's bottom right, inside the canvas: it flips.
        let mut corner = Some(ObjectMenuState::new(agent, true, (530, 380)));
        let (_, rect) = click_row(&mut corner, 0);
        assert_eq!(
            rect.max,
            egui::pos2(
                layout.canvas.x + 530.0 * layout.scale,
                layout.canvas.y + 380.0 * layout.scale
            )
        );
    }
}
