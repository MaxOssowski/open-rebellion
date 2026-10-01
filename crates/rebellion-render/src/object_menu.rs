//! The object pop-up menu a right-click on a character or special force opens
//! in a system window (`FUN_004ac5c0`). Recovery notes:
//! `ghidra/notes/object-popup-menu.md`.
//!
//! `FUN_0051d990` lists the orders the selection's class offers, sorts them
//! by their STRATEGY `RT_RCDATA` record's key, and always adds Encyclopedia
//! and Status. Each record names its text (word 5, TEXTSTRA), its parent
//! submenu (word 1) and its sort key (word 2).

use rebellion_core::missions::MissionMember;

use crate::game_menu::GameMenuEntry;

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
/// force's only the first four (`FUN_00503fa0`). Kinds `0x204`, `0x241` and
/// `0x268` are offered too but have no STRATEGY record, so they never show.
/// An empty selection lists only Encyclopedia and Status, both disabled.
///
/// - Mission is enabled when `mission_enabled` says so
///   (`MissionState::mission_order_enabled`). port: the global gate
///   `FUN_0051de80` is taken as clear.
/// - Encyclopedia is enabled for a single selection.
/// - port: Move, Confirmed Move, Command, Status and Retire stay disabled
///   until their windows and orders are ported. In the original, Status is
///   enabled for a single selection that is not a system, and Command is a
///   submenu parent.
#[must_use]
pub fn object_menu_rows(
    selection: Option<MissionMember>,
    mission_enabled: bool,
) -> Vec<ObjectMenuRow> {
    let offered: &[ObjectMenuItem] = match selection {
        Some(MissionMember::Character(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION, COMMAND],
        Some(MissionMember::SpecialForce(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION],
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

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::ids::{CharacterKey, SpecialForceKey};

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
        let rows = object_menu_rows(
            Some(MissionMember::Character(CharacterKey::default())),
            true,
        );
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
            Some(MissionMember::SpecialForce(SpecialForceKey::default())),
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
    fn mission_follows_the_order_rule_and_encyclopedia_needs_one_selection() {
        // FUN_0051fe20 enables Mission; FUN_0051d990 enables Encyclopedia
        // for a single selection.
        let character = Some(MissionMember::Character(CharacterKey::default()));
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
        let rows = object_menu_rows(
            Some(MissionMember::Character(CharacterKey::default())),
            true,
        );
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
}
