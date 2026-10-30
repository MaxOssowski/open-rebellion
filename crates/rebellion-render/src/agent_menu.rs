//! The Agent menu (`ghidra/notes/agent-menu.md`): STRATEGY `RT_RCDATA` list
//! `0xdead`, items `0x110..0x11e`. A right click on the advisor panel opens
//! it (`FUN_00422ce0` → `FUN_004420b0` → `FUN_0042d050`), and
//! `FUN_00487900` enables and checks its items before it shows.

use egui_macroquad::egui;

use crate::bmp_cache::BmpCache;
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::game_menu::{
    GameMenuEntry, GameMenuPlacement, GameMenuResponse, GameMenuState, GameSubmenu, draw_game_menu,
    draw_game_menu_with_submenu,
};

const AGENT_MENU_ID: &str = "original-agent-menu";

/// The bitmap each record names in word 10, shown for a checked item.
/// hyp: word 10 is the checked item's bitmap, as the Command submenu's
/// ranks carry it (`object-popup-menu.md`).
const CHECK_BITMAP: u32 = 11902;

/// An Agent menu command the port carries out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCommand {
    /// 0x115, Alt+G: toggle module 0x15 (`FUN_00439d60`).
    ManageGarrisons,
    /// 0x116, Alt+U: toggle module 0x14 (`FUN_00439d60`).
    ManageProduction,
    /// 0x11e, Alt+A: flip bit `0x8000` (`FUN_00439e80`).
    AgentAdvice,
    /// 0x114, Alt+H: the Status window on the empty id, the Objectives
    /// (`FUN_00486fb0` → `FUN_0041d7f0(FUN_004ece30())`).
    Objectives,
    /// 0x113, Alt+O: the Status window on the galaxy, the Galaxy Overview
    /// (`FUN_00486fb0` → `FUN_0041d7f0(0xf1000004)`).
    GalaxyOverview,
    /// 0x11b, Alt+V: flip bit `0x1000`, Translate Counterpart
    /// (`FUN_00439e80`).
    TranslateCounterpart,
}

/// One record of list `0xdead`: its command, sort key (word 2) and TEXTSTRA
/// label (word 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMenuRecord {
    pub command: u16,
    pub sort_key: u16,
    pub label: &'static str,
}

/// STRATEGY `RT_RCDATA` `0x110..0x11e` (no `0x11d` in the list), in sort
/// order; labels are TEXTSTRA `0x3000 + command`.
pub const AGENT_MENU_RECORDS: [AgentMenuRecord; 14] = [
    AgentMenuRecord {
        command: 0x110,
        sort_key: 100,
        label: "Build Ships",
    },
    AgentMenuRecord {
        command: 0x111,
        sort_key: 110,
        label: "Build Troops",
    },
    AgentMenuRecord {
        command: 0x112,
        sort_key: 120,
        label: "Build Facilities",
    },
    AgentMenuRecord {
        command: 0x113,
        sort_key: 200,
        label: "Galaxy Overview",
    },
    AgentMenuRecord {
        command: 0x114,
        sort_key: 500,
        label: "Objectives",
    },
    AgentMenuRecord {
        command: 0x115,
        sort_key: 510,
        label: "Manage Garrisons",
    },
    AgentMenuRecord {
        command: 0x116,
        sort_key: 520,
        label: "Manage Production",
    },
    AgentMenuRecord {
        command: 0x117,
        sort_key: 530,
        label: "Manage Maintenance",
    },
    AgentMenuRecord {
        command: 0x118,
        sort_key: 540,
        label: "Not An Operation",
    },
    AgentMenuRecord {
        command: 0x119,
        sort_key: 600,
        label: "Deactivate",
    },
    AgentMenuRecord {
        command: 0x11a,
        sort_key: 601,
        label: "Reactivate",
    },
    AgentMenuRecord {
        command: 0x11b,
        sort_key: 610,
        label: "Translate Counterpart",
    },
    AgentMenuRecord {
        command: 0x11e,
        sort_key: 615,
        label: "Agent Advice",
    },
    AgentMenuRecord {
        command: 0x11c,
        sort_key: 620,
        label: "Messages",
    },
];

/// The agent state the menu shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentMenuView {
    pub garrisons: bool,
    pub production: bool,
    pub advice: bool,
    /// Bit `0x1000`: Translate Counterpart is checked.
    pub translate: bool,
}

/// One item as `FUN_00487900` leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMenuItem {
    pub record: AgentMenuRecord,
    pub enabled: bool,
    pub checked: bool,
    pub command: Option<AgentCommand>,
}

/// The items `FUN_00487900` puts in the menu (`FUN_00520690` adds each
/// one), in their records' sort order: the other records of list `0xdead`
/// never show (A0 2026-10-29, nine items).
const MENU_COMMANDS: [u16; 9] = [0x110, 0x111, 0x112, 0x115, 0x116, 0x113, 0x11b, 0x114, 0x11e];

/// The items `FUN_00487900` adds, enables and checks. Manage Garrisons and
/// Manage Production are checked while they run (`FUN_00439e30`), Agent
/// Advice while bit `0x8000` is clear, Translate Counterpart while
/// `0x1000` is set. port: the Build orders (module 0x17) are not ported,
/// so they stay disabled.
#[must_use]
pub fn agent_menu_items(view: AgentMenuView) -> Vec<AgentMenuItem> {
    AGENT_MENU_RECORDS
        .iter()
        .filter(|record| MENU_COMMANDS.contains(&record.command))
        .map(|&record| {
            let (command, checked) = match record.command {
                0x115 => (Some(AgentCommand::ManageGarrisons), view.garrisons),
                0x116 => (Some(AgentCommand::ManageProduction), view.production),
                0x113 => (Some(AgentCommand::GalaxyOverview), false),
                0x11b => (Some(AgentCommand::TranslateCounterpart), view.translate),
                0x114 => (Some(AgentCommand::Objectives), false),
                0x11e => (Some(AgentCommand::AgentAdvice), view.advice),
                _ => (None, false),
            };
            AgentMenuItem {
                record,
                enabled: command.is_some(),
                checked,
                command,
            }
        })
        .collect()
}

/// Whether a right click at canvas `point` lands on the agent's panel.
/// hyp: the agent is the larger droid aperture (`FUN_0042adb0`, panel
/// `param_1[0x4a]`); the other panel's list is `0xbeef`, the Message Alerts.
#[must_use]
pub fn agent_panel_contains(faction: CockpitFaction, point: (f32, f32)) -> bool {
    let (x, y, width, height) = crate::advisor::agent_aperture(faction);
    point.0 >= x && point.0 < x + width && point.1 >= y && point.1 < y + height
}

/// The open menu's anchor, a 640 by 480 canvas point.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AgentMenuState {
    pub anchor: Option<(f32, f32)>,
}

/// Open the menu on a right-button release over the agent's panel
/// (`FUN_00422ce0` `WM_RBUTTONUP` → `FUN_004420b0`), anchored at the
/// release. A release over a window or menu does not reach the panel.
pub fn open_agent_menu_on_right_click(
    ctx: &egui::Context,
    state: &mut AgentMenuState,
    layout: CockpitLayout,
    faction: CockpitFaction,
) -> bool {
    if layout.scale <= 0.0 || ctx.is_pointer_over_area() {
        return false;
    }
    let (released, pointer) = ctx.input(|input| {
        (
            input
                .pointer
                .button_released(egui::PointerButton::Secondary),
            input.pointer.interact_pos(),
        )
    });
    let Some(pointer) = pointer.filter(|_| released) else {
        return false;
    };
    let point = (
        (pointer.x - layout.canvas.x) / layout.scale,
        (pointer.y - layout.canvas.y) / layout.scale,
    );
    if !agent_panel_contains(faction, point) {
        return false;
    }
    state.anchor = Some(point);
    true
}

/// Draw the open Agent menu and return the command chosen.
pub fn draw_agent_menu(
    ctx: &egui::Context,
    state: &mut AgentMenuState,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    view: AgentMenuView,
    input_enabled: bool,
) -> Option<AgentCommand> {
    let anchor = state.anchor?;
    let items = agent_menu_items(view);
    let entries: Vec<GameMenuEntry<'_>> = items
        .iter()
        .map(|item| GameMenuEntry {
            label: item.record.label,
            icon: item.checked.then_some(CHECK_BITMAP),
            enabled: item.enabled,
            submenu: false,
        })
        .collect();
    match draw_game_menu(
        ctx,
        egui::Id::new(AGENT_MENU_ID),
        cache,
        layout,
        faction,
        GameMenuPlacement::in_frame(anchor),
        &entries,
        input_enabled,
    ) {
        GameMenuResponse::Open | GameMenuResponse::ChosenChild(_) => None,
        GameMenuResponse::Dismissed => {
            state.anchor = None;
            None
        }
        GameMenuResponse::Chosen(index) => {
            state.anchor = None;
            items[index].command
        }
    }
}

// ---------------------------------------------------------------------------
// Message Alerts (list 0xbeef)
// ---------------------------------------------------------------------------

const ALERTS_MENU_ID: &str = "original-message-alerts-menu";

/// One Message Alerts record: its command, sort key (word 2), TEXTSTRA
/// label (word 5) and the message category mask it toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertRecord {
    pub command: u16,
    pub sort_key: u16,
    pub label: &'static str,
    pub mask: u16,
}

/// The submenu "Message Alerts" (STRATEGY `RT_RCDATA` `0x170`, sort key 630,
/// TEXTSTRA 12573), parent of `0x171..0x179` (word 1 `0x170`), in sort
/// order; the masks are `FUN_00486fb0`'s.
pub const ALERT_RECORDS: [AlertRecord; 9] = [
    AlertRecord {
        command: 0x171,
        sort_key: 631,
        label: "Loyalty",
        mask: 0x001,
    },
    AlertRecord {
        command: 0x172,
        sort_key: 632,
        label: "Fleets",
        mask: 0x080,
    },
    AlertRecord {
        command: 0x173,
        sort_key: 633,
        label: "Mission",
        mask: 0x010,
    },
    AlertRecord {
        command: 0x174,
        sort_key: 634,
        label: "Resources",
        mask: 0x004,
    },
    AlertRecord {
        command: 0x175,
        sort_key: 635,
        label: "Manufacturing",
        mask: 0x008,
    },
    AlertRecord {
        command: 0x176,
        sort_key: 636,
        label: "Defense",
        mask: 0x040,
    },
    AlertRecord {
        command: 0x177,
        sort_key: 637,
        label: "Conflict",
        mask: 0x100,
    },
    AlertRecord {
        command: 0x179,
        sort_key: 638,
        label: "Chat",
        mask: 0x020,
    },
    AlertRecord {
        command: 0x178,
        sort_key: 639,
        label: "Advice",
        mask: 0x200,
    },
];

/// What the Message Alerts menu asks of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertsCommand {
    /// `0x11c` "Messages": the Message Index on All (`FUN_0041d770(1, 0x79)`).
    Messages,
    /// `0x171..0x179`: flip the category's silent bit (`FUN_00486fb0`:
    /// `FUN_0048a210(mask, FUN_0048a1c0(mask) ^ 4)`).
    ToggleAlert(u16),
}

/// The open Message Alerts menu: its anchor, a 640 by 480 canvas point,
/// and its window's submenu state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AlertsMenuState {
    pub anchor: Option<(f32, f32)>,
    window: GameMenuState,
}

/// Whether a right click at canvas `point` lands on the second droid's
/// panel (list `0xbeef`, `FUN_0042d050`).
#[must_use]
pub fn alerts_panel_contains(faction: CockpitFaction, point: (f32, f32)) -> bool {
    let (x, y, width, height) = crate::advisor::second_aperture(faction);
    point.0 >= x && point.0 < x + width && point.1 >= y && point.1 < y + height
}

/// Open the Message Alerts menu on a right-button release over the second
/// droid's panel (`FUN_0042d050` with `0xbeef` → `FUN_0041d190` →
/// `FUN_00487a60`), anchored at the release.
pub fn open_alerts_menu_on_right_click(
    ctx: &egui::Context,
    state: &mut AlertsMenuState,
    layout: CockpitLayout,
    faction: CockpitFaction,
) -> bool {
    if layout.scale <= 0.0 || ctx.is_pointer_over_area() {
        return false;
    }
    let (released, pointer) = ctx.input(|input| {
        (
            input
                .pointer
                .button_released(egui::PointerButton::Secondary),
            input.pointer.interact_pos(),
        )
    });
    let Some(pointer) = pointer.filter(|_| released) else {
        return false;
    };
    let point = (
        (pointer.x - layout.canvas.x) / layout.scale,
        (pointer.y - layout.canvas.y) / layout.scale,
    );
    if !alerts_panel_contains(faction, point) {
        return false;
    }
    *state = AlertsMenuState {
        anchor: Some(point),
        window: GameMenuState::default(),
    };
    true
}

/// The submenu's entries as `FUN_00487a60` leaves them: every category
/// enabled, checked while it alerts (its option's bit 4 clear).
#[must_use]
pub fn alert_entries(silent: impl Fn(u16) -> bool) -> Vec<(AlertRecord, bool)> {
    ALERT_RECORDS
        .iter()
        .map(|&record| (record, !silent(record.mask)))
        .collect()
}

/// Draw the open Message Alerts menu: "Messages" (sort key 620) and the
/// "Message Alerts" submenu (630); return the command chosen.
pub fn draw_alerts_menu(
    ctx: &egui::Context,
    state: &mut AlertsMenuState,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    silent: impl Fn(u16) -> bool,
    input_enabled: bool,
) -> Option<AlertsCommand> {
    let anchor = state.anchor?;
    let entries = [
        GameMenuEntry {
            label: "Messages",
            icon: None,
            enabled: true,
            submenu: false,
        },
        GameMenuEntry {
            label: "Message Alerts",
            icon: None,
            enabled: true,
            submenu: true,
        },
    ];
    let alerts = alert_entries(silent);
    let children: Vec<GameMenuEntry<'_>> = alerts
        .iter()
        .map(|(record, checked)| GameMenuEntry {
            label: record.label,
            icon: checked.then_some(CHECK_BITMAP),
            enabled: true,
            submenu: false,
        })
        .collect();
    match draw_game_menu_with_submenu(
        ctx,
        egui::Id::new(ALERTS_MENU_ID),
        cache,
        layout,
        faction,
        GameMenuPlacement::in_frame(anchor),
        &entries,
        Some(GameSubmenu {
            parent: 1,
            entries: &children,
        }),
        &mut state.window,
        input_enabled,
    ) {
        GameMenuResponse::Open => None,
        response => {
            state.anchor = None;
            alerts_command(response)
        }
    }
}

/// The command a closing response chooses: row 0 is "Messages"; the
/// parent row chooses nothing; a submenu row toggles its category.
#[must_use]
pub fn alerts_command(response: GameMenuResponse) -> Option<AlertsCommand> {
    match response {
        GameMenuResponse::Chosen(0) => Some(AlertsCommand::Messages),
        GameMenuResponse::ChosenChild(index) => ALERT_RECORDS
            .get(index)
            .map(|record| AlertsCommand::ToggleAlert(record.mask)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alerts_submenu_lists_its_categories_in_sort_order_checked_while_they_alert() {
        // STRATEGY RCDATA 0x171..0x179 (parent 0x170): sort keys 631..639,
        // Chat (638) before Advice (639); FUN_00487a60 checks a category
        // while its option's bit 4 is clear; FUN_00486fb0's masks.
        let keys: Vec<u16> = ALERT_RECORDS.iter().map(|record| record.sort_key).collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            ALERT_RECORDS.map(|record| (record.command, record.mask)),
            [
                (0x171, 0x001),
                (0x172, 0x080),
                (0x173, 0x010),
                (0x174, 0x004),
                (0x175, 0x008),
                (0x176, 0x040),
                (0x177, 0x100),
                (0x179, 0x020),
                (0x178, 0x200),
            ]
        );
        let entries = alert_entries(|mask| mask == 0x200);
        assert!(
            entries
                .iter()
                .all(|(record, checked)| *checked == (record.mask != 0x200))
        );
        assert_eq!(entries[8].0.label, "Advice");
    }

    #[test]
    fn the_alerts_menu_opens_the_index_or_toggles_the_chosen_category() {
        // FUN_00486fb0: 0x11c opens the index on All; 0x171..0x179 flip
        // their category's bit 4.
        assert_eq!(
            alerts_command(GameMenuResponse::Chosen(0)),
            Some(AlertsCommand::Messages)
        );
        assert_eq!(alerts_command(GameMenuResponse::Chosen(1)), None);
        assert_eq!(alerts_command(GameMenuResponse::Dismissed), None);
        assert_eq!(
            alerts_command(GameMenuResponse::ChosenChild(0)),
            Some(AlertsCommand::ToggleAlert(0x001))
        );
        assert_eq!(
            alerts_command(GameMenuResponse::ChosenChild(8)),
            Some(AlertsCommand::ToggleAlert(0x200))
        );
        assert_eq!(alerts_command(GameMenuResponse::ChosenChild(9)), None);
    }

    #[test]
    fn the_second_droids_panel_opens_the_alerts_menu() {
        // FUN_0042d050: 0xbeef is the second droid's panel (FUN_0042adb0):
        // Alliance R2-D2 (316, 411) 47 by 69, Empire SD-7 (302, 401) 101
        // by 79.
        assert!(alerts_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 420.0)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Alliance,
            (364.0, 420.0)
        ));
        assert!(alerts_panel_contains(
            CockpitFaction::Empire,
            (402.0, 479.0)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Empire,
            (301.0, 420.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 420.0)
        ));
        // The right and bottom edges are outside.
        assert!(alerts_panel_contains(
            CockpitFaction::Alliance,
            (362.9, 479.9)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Alliance,
            (363.0, 420.0)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 480.0)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Alliance,
            (315.9, 420.0)
        ));
        assert!(!alerts_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 410.9)
        ));
    }

    /// A screen that scales the canvas by 2 and letterboxes it 20 down,
    /// and one that letterboxes it sideways.
    const TEST_SCREEN: (f32, f32) = (1280.0, 1000.0);
    const WIDE_SCREEN: (f32, f32) = (1400.0, 1000.0);

    /// Run `frames` through the alerts menu at the Alliance's layout on
    /// `screen`: each frame's pointer and a button pressed or released
    /// there; returns each frame's opening and command.
    fn drive_alerts(
        screen: (f32, f32),
        state: &mut AlertsMenuState,
        frames: &[(egui::Pos2, Option<(egui::PointerButton, bool)>)],
    ) -> Vec<(bool, Option<AlertsCommand>)> {
        let placed: Vec<_> = frames
            .iter()
            .map(|&(pos, button)| (Placed::At(pos), button))
            .collect();
        drive_alerts_placed(screen, state, &placed)
    }

    /// A pointer at a screen point, or on the open menu's row `n` of two.
    #[derive(Clone, Copy)]
    enum Placed {
        At(egui::Pos2),
        Row(usize),
    }

    fn drive_alerts_placed(
        screen: (f32, f32),
        state: &mut AlertsMenuState,
        frames: &[(Placed, Option<(egui::PointerButton, bool)>)],
    ) -> Vec<(bool, Option<AlertsCommand>)> {
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let layout = crate::cockpit::CockpitState::new(CockpitFaction::Alliance)
            .layout_for(screen.0, screen.1);
        let mut out = Vec::new();
        for (index, &(placed, button)) in frames.iter().enumerate() {
            let pos = match placed {
                Placed::At(pos) => pos,
                Placed::Row(row) => {
                    let menu = ctx
                        .memory(|memory| memory.area_rect(egui::Id::new(ALERTS_MENU_ID)))
                        .expect("the menu is shown");
                    #[expect(clippy::cast_precision_loss, reason = "Two rows.")]
                    let y = menu.top() + menu.height() * (row as f32 * 2.0 + 1.0) / 4.0;
                    egui::pos2(menu.center().x, y)
                }
            };
            let mut events = vec![egui::Event::PointerMoved(pos)];
            if let Some((button, pressed)) = button {
                events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
            #[expect(clippy::cast_precision_loss, reason = "A few frames.")]
            let time = index as f64 * 0.05;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(screen.0, screen.1),
                )),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut step = (false, None);
            let _ = ctx.run(input, |ctx| {
                step.0 =
                    open_alerts_menu_on_right_click(ctx, state, layout, CockpitFaction::Alliance);
                step.1 = draw_alerts_menu(
                    ctx,
                    state,
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    |_| false,
                    true,
                );
            });
            out.push(step);
        }
        out
    }

    #[test]
    fn a_right_click_on_the_second_droid_opens_the_menu_at_the_release() {
        // FUN_0042d050 (0xbeef): a right-button release on the panel opens
        // the menu at the release; elsewhere it does not.
        for screen in [TEST_SCREEN, WIDE_SCREEN] {
            opens_at_the_release(screen);
        }
    }

    fn opens_at_the_release(screen: (f32, f32)) {
        let layout = crate::cockpit::CockpitState::new(CockpitFaction::Alliance)
            .layout_for(screen.0, screen.1);
        let at = |x: f32, y: f32| {
            egui::pos2(
                layout.canvas.x + x * layout.scale,
                layout.canvas.y + y * layout.scale,
            )
        };
        let click = |point: egui::Pos2, button| {
            [
                (point, Some((button, true))),
                (point, Some((button, false))),
            ]
        };
        let mut state = AlertsMenuState::default();
        let outside = drive_alerts(
            screen,
            &mut state,
            &click(at(200.0, 200.0), egui::PointerButton::Secondary),
        );
        assert_eq!(outside, [(false, None), (false, None)]);
        assert_eq!(state.anchor, None);
        let left = drive_alerts(
            screen,
            &mut state,
            &click(at(339.0, 445.0), egui::PointerButton::Primary),
        );
        assert_eq!(
            left,
            [(false, None), (false, None)],
            "a left click does not open it"
        );
        let opened = drive_alerts(
            screen,
            &mut state,
            &click(at(339.0, 445.0), egui::PointerButton::Secondary),
        );
        assert!(opened[1].0);
        let anchor = state.anchor.expect("the menu opens");
        assert!(
            (anchor.0 - 339.0).abs() < 0.01 && (anchor.1 - 445.0).abs() < 0.01,
            "{anchor:?}"
        );
    }

    #[test]
    fn the_menus_first_row_opens_the_message_index() {
        // FUN_00486fb0 0x11c: FUN_0041d770(1, 0x79).
        let layout = crate::cockpit::CockpitState::new(CockpitFaction::Alliance)
            .layout_for(TEST_SCREEN.0, TEST_SCREEN.1);
        let droid = egui::pos2(
            layout.canvas.x + 339.0 * layout.scale,
            layout.canvas.y + 445.0 * layout.scale,
        );
        let right = egui::PointerButton::Secondary;
        let left = egui::PointerButton::Primary;
        let mut state = AlertsMenuState::default();
        let out = drive_alerts_placed(
            TEST_SCREEN,
            &mut state,
            &[
                (Placed::At(droid), Some((right, true))),
                (Placed::At(droid), Some((right, false))),
                (Placed::Row(0), None),
                (Placed::Row(0), Some((left, true))),
                (Placed::Row(0), Some((left, false))),
            ],
        );
        assert_eq!(
            out.last(),
            Some(&(false, Some(AlertsCommand::Messages))),
            "{out:?}"
        );
        assert_eq!(state.anchor, None, "the menu closes");
    }

    #[test]
    fn the_menu_lists_list_0xdeads_records_in_sort_order() {
        // STRATEGY RT_RCDATA 0x110..0x11e, word 2 the sort key, word 5 the
        // TEXTSTRA label; 0x11d (Message Alerts) is not in the list.
        let keys: Vec<u16> = AGENT_MENU_RECORDS
            .iter()
            .map(|record| record.sort_key)
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        assert!(!AGENT_MENU_RECORDS
            .iter()
            .any(|record| record.command == 0x11d));
        assert_eq!(AGENT_MENU_RECORDS[12].label, "Agent Advice");
    }

    #[test]
    fn running_automations_and_advice_are_checked_and_enabled() {
        // FUN_00487900: 0x115/0x116 checked when FUN_00439e30 says the module
        // runs; 0x11e checked when bit 0x8000 is clear.
        let item = |items: &[AgentMenuItem], command| {
            *items
                .iter()
                .find(|item| item.record.command == command)
                .unwrap()
        };
        let off = agent_menu_items(AgentMenuView::default());
        for command in [0x115, 0x116, 0x11e] {
            assert!(item(&off, command).enabled && !item(&off, command).checked);
        }
        let on = agent_menu_items(AgentMenuView {
            garrisons: true,
            production: false,
            advice: true,
            translate: false,
        });
        assert!(item(&on, 0x115).checked);
        assert!(!item(&on, 0x116).checked);
        assert!(item(&on, 0x11e).checked);
        assert_eq!(item(&on, 0x11e).command, Some(AgentCommand::AgentAdvice));
        assert!(!item(&on, 0x110).enabled, "port: Build Ships is not ported");
        assert_eq!(item(&on, 0x114).command, Some(AgentCommand::Objectives));
        assert!(item(&off, 0x114).enabled, "FUN_00487900 enables 0x114 always");
        assert_eq!(item(&on, 0x113).command, Some(AgentCommand::GalaxyOverview));
        assert!(item(&off, 0x113).enabled, "FUN_00487900 enables 0x113 always");
        // 0x11b: enabled with the agent, checked while 0x1000 is set.
        assert!(item(&off, 0x11b).enabled && !item(&off, 0x11b).checked);
        let translating = agent_menu_items(AgentMenuView {
            translate: true,
            ..AgentMenuView::default()
        });
        assert!(item(&translating, 0x11b).checked);
        assert_eq!(item(&translating, 0x11b).command, Some(AgentCommand::TranslateCounterpart));
    }

    #[test]
    fn the_menu_holds_the_nine_items_fun_00487900_adds_in_sort_order() {
        // FUN_00487900 adds 0x110..0x116 but 0x117/0x118, 0x11b, 0x114 and
        // 0x11e; the A0 capture of the Empire's menu shows these nine.
        let labels: Vec<_> = agent_menu_items(AgentMenuView::default())
            .iter()
            .map(|item| item.record.label)
            .collect();
        assert_eq!(
            labels,
            [
                "Build Ships",
                "Build Troops",
                "Build Facilities",
                "Galaxy Overview",
                "Objectives",
                "Manage Garrisons",
                "Manage Production",
                "Translate Counterpart",
                "Agent Advice",
            ]
        );
    }

    #[test]
    fn a_right_click_on_the_agents_droid_opens_the_menu() {
        // FUN_004420b0 hit-tests the panel at the click; the larger droid's
        // aperture (FUN_0042adb0).
        assert!(agent_panel_contains(
            CockpitFaction::Alliance,
            (560.0, 400.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 420.0)
        ));
        assert!(agent_panel_contains(CockpitFaction::Empire, (50.0, 400.0)));
        // (541, 337, 67, 116): right and bottom edges are outside.
        assert!(agent_panel_contains(
            CockpitFaction::Alliance,
            (541.0, 337.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (608.0, 400.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (560.0, 453.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (540.0, 400.0)
        ));
    }

    /// A right click's release at `screen` on a canvas at (10, 20), scale 2.
    fn right_click(state: &mut AgentMenuState, screen: egui::Pos2, scale: f32) -> bool {
        use crate::cockpit::CockpitViewport;
        let layout = CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 640.0 * scale,
                height: 480.0 * scale,
            },
            galaxy: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            scale,
        };
        let ctx = egui::Context::default();
        let mut opened = false;
        for pressed in [true, false] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(screen),
                    egui::Event::PointerButton {
                        pos: screen,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                opened |=
                    open_agent_menu_on_right_click(ctx, state, layout, CockpitFaction::Alliance);
            });
        }
        opened
    }

    #[test]
    fn a_right_button_release_on_the_droid_anchors_the_menu_at_its_canvas_point() {
        // FUN_00422ce0 WM_RBUTTONUP -> FUN_004420b0 at the release point.
        let mut state = AgentMenuState::default();
        assert!(right_click(
            &mut state,
            egui::pos2(10.0 + 560.0 * 2.0, 20.0 + 400.0 * 2.0),
            2.0
        ));
        assert_eq!(state.anchor, Some((560.0, 400.0)));

        let mut state = AgentMenuState::default();
        assert!(!right_click(
            &mut state,
            egui::pos2(10.0 + 320.0 * 2.0, 20.0 + 420.0 * 2.0),
            2.0
        ));
        assert_eq!(state.anchor, None);

        let mut state = AgentMenuState::default();
        assert!(!right_click(&mut state, egui::pos2(570.0, 420.0), 0.0));
        assert_eq!(state.anchor, None);
    }
}
