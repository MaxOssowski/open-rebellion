//! The original Missions window (window type 11, `FUN_0049f130`): the
//! missions under way at a system, the selected mission's agents or decoys,
//! and its target. Recovery notes: `ghidra/notes/sector-quadrants.md`,
//! "Type 11".
//!
//! It opens from the icon a sector window shows at the bottom right of a
//! planet (`FUN_0045d090`, `FUN_0045aac0` kind `0x40`), and a move released
//! over the window targets its system (`+0x70`, `FUN_004aa470`).

use std::collections::BTreeMap;

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{CharacterKey, SystemKey};
use rebellion_core::missions::{
    ActiveMission, MissionFaction, MissionMember, MissionState, MissionTarget,
};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::defenses_window::title_resource;
use crate::fleet_window::{
    faction_side, mini_marks, paint_key_pixel, paint_native, KeyPixel, MiniObject,
};
use crate::mission_dialog::{kind_icon, kind_name};
use crate::minis::capital_ship_mini_id;
use crate::quadrant_icons::{mission_keys, missions_side, visible_members};
use crate::sector_window::planet_resource_id;
use crate::system_window::{
    character_picture, clamp_window_to_galaxy, defense_facility_mini, exact_clicked, logical_rect,
    manufacturing_facility_mini, production_facility_mini, rect_contains, special_force_mini,
    troop_mini,
};
use crate::window_move::{self, MoveAnchor};

pub const MISSIONS_WINDOW_WIDTH: f32 = 235.0;
pub const MISSIONS_WINDOW_HEIGHT: f32 = 304.0;

const BACKGROUND: u32 = 11165;
const CLOSE_NORMAL: u32 = 10108;
const CLOSE_PRESSED: u32 = 10109;
const MINIMIZE_NORMAL: u32 = 10253;
const MINIMIZE_PRESSED: u32 = 10254;
const SECTOR_NORMAL: u32 = 10209;
const SECTOR_PRESSED: u32 = 10208;
/// The frame keyed over a selected mission's picture (`FUN_004a1590`):
/// 11127 while the player is side 1, 11128 otherwise.
const MISSION_FRAME: [u32; 2] = [11127, 11128];
/// A minimized Missions window's rail icon (`FUN_004a21c0`).
const RAIL_ICONS: [u32; 3] = [11539, 11540, 11541];
/// A mission's GOKRES mini is its class's TEXTSTRA id's low 12 bits plus
/// `0x4000` (`FUN_004a1590`); [`kind_icon`] adds `0x1000` for side 2.
const MISSION_MINI_BASE: u32 = 0x4000;

/// The title label `+0x17c` (`FUN_0049fef0`): after the strip's corner (2, 2)
/// and the sector button's width plus 5, 235 less three times 19 wide, 16
/// high.
const TITLE: (f32, f32, f32, f32) = (21.0, 2.0, 178.0, 16.0);
/// The mission list `+0x1b8` (`FUN_00607ea0`, id 10) and its 90 by 50 rows,
/// stacked downwards (`+0xf0` 1, no grid).
const MISSION_LIST: (f32, f32, f32, f32) = (5.0, 24.0, 94.0, 275.0);
const MISSION_ROW: (f32, f32) = (90.0, 50.0);
/// A list item's text rect: the cell moved by the list's (`+0xe8`, `+0xec`)
/// default (1, 0), trimmed by 3 on the right for a left-aligned format.
const TEXT_OFFSET: (f32, f32) = (1.0, 0.0);
const TEXT_RIGHT_TRIM: f32 = 3.0;
/// The tab strip (`FUN_0060d590`, id `0x16`) and its two 61 by 16 buttons.
const TAB_STRIP: (f32, f32) = (105.0, 127.0);
const TAB_SIZE: (f32, f32) = (61.0, 16.0);
/// The member list `+0x1c0` (id `0xb`): a grid of 115 by 43 cells, one to a
/// row at this width, each item a 122 by 43 image.
const MEMBER_LIST: (f32, f32, f32, f32) = (107.0, 145.0, 117.0, 147.0);
const MEMBER_ROW: (f32, f32) = (115.0, 43.0);
const MEMBER_IMAGE: (f32, f32) = (122.0, 43.0);
/// The target picture's box (`FUN_0049fef0`, `FUN_004a10a0`).
const PICTURE: (f32, f32, f32, f32) = (108.0, 37.0, 122.0, 50.0);
/// `+0x178`, TEXTSTRA 34081 "Target:", and `+0x174`, the target's name.
const TARGET_LABEL: (f32, f32, f32) = (109.0, 25.0, 113.0);
const TARGET_NAME: (f32, f32, f32) = (109.0, 91.0, 113.0);

/// The member list's two tabs (`FUN_004a1ba0`), by button id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionsTab {
    /// Id `0x17`: members whose role `+0x78` bit 0 is clear.
    Agents,
    /// Id `0x18`: the decoys, bit 0 set.
    Decoys,
}

impl MissionsTab {
    /// The tab's tooltip (`FUN_0049f540`, TEXTSTRA): 0x8500 "Agents",
    /// 0x8501 "Decoys".
    #[must_use]
    pub const fn tooltip_string(self) -> u16 {
        match self {
            Self::Agents => 0x8500,
            Self::Decoys => 0x8501,
        }
    }

    pub const ALL: [Self; 2] = [Self::Agents, Self::Decoys];

    /// The button's x in the strip (`FUN_0049f540`).
    const fn x(self) -> f32 {
        match self {
            Self::Agents => 0.0,
            Self::Decoys => 61.0,
        }
    }

    /// The help message: TEXTSTRA 34048 and 34049.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Agents => "Agents",
            Self::Decoys => "Decoys",
        }
    }
}

/// The tab's bitmap by the selected mission's side (`FUN_004a0ca0`): the
/// normal id, or the pressed id one higher while the tab is selected
/// (`FUN_0060d700` copies it into the unset selected state). A side other
/// than 1 or 2 sets no art.
#[must_use]
pub const fn tab_resource(tab: MissionsTab, side: u8, selected: bool) -> Option<u32> {
    let base = match (tab, side) {
        (MissionsTab::Agents, 1) => 11560,
        (MissionsTab::Decoys, 1) => 11562,
        (MissionsTab::Agents, 2) => 11565,
        (MissionsTab::Decoys, 2) => 11567,
        _ => return None,
    };
    Some(if selected { base + 1 } else { base })
}

/// One mission row (`FUN_004a1590`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct MissionRow {
    mission: u64,
    /// The side bits of the first member seen carrying the mission.
    side: u8,
    picture: u32,
    label: &'static str,
}

/// One member row (`FUN_004a0e10`): the member, its GOKRES mini (`None`
/// where the port maps none) and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemberRow {
    member: MissionMember,
    mini: Option<u32>,
    label: String,
    /// The marks drawn under its mini (`mini_marks`): en route to the
    /// mission's target (`+0x50` bit 4), damaged while a character is
    /// injured (bit `0x200`, `FUN_004f7800`).
    marks: Vec<(DllSource, u32)>,
}

/// A character member while injured (`FUN_004f7800` sets `+0x50` bit
/// `0x200` to injury != 0). port: a special force keeps no injury.
fn member_injured(world: &GameWorld, member: MissionMember) -> bool {
    match member {
        MissionMember::Character(key) => world
            .characters
            .get(key)
            .is_some_and(|character| character.injury > 0),
        MissionMember::SpecialForce(_) => false,
    }
}

/// What the selected mission's details name (`+0x1c8`, `FUN_004a10a0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    System(SystemKey),
    Character(CharacterKey),
    Object(MissionTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenMissionsWindow {
    system: SystemKey,
    logical_position: (i16, i16),
    /// The selected mission row `+0x1c4`, by mission id.
    selected: Option<u64>,
    tab: MissionsTab,
    /// Where the window was when a press on its surface took it
    /// (`crate::window_move`).
    moving: MoveAnchor,
}

/// The open Missions windows. The last is focused and paints on top.
#[derive(Debug)]
pub struct MissionsWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenMissionsWindow>,
    /// The kind's claim on the shared window focus.
    focus: crate::window_focus::FocusStamp,
    /// Minimized windows (`0x466`): hidden, not destroyed, so one comes
    /// back with its selected mission and tab. `FUN_00428b40` keeps every
    /// window kind's object in its rail slot alike (A0 personnel-states/39,
    /// the Fleet window's). port: a hidden one whose rail slot was taken
    /// stays here until its system's next open.
    hidden: Vec<OpenMissionsWindow>,
    /// A window was created, not raised, since [`Self::take_created`]
    /// (`FUN_0045aac0` then calls `FUN_0041d2d0`).
    created: bool,
}

impl Default for MissionsWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            focus: crate::window_focus::FocusStamp::default(),
            hidden: Vec::new(),
            created: false,
        }
    }
}

impl MissionsWindowState {
    /// When this kind last raised a window, while one is open: its claim
    /// on the shared focus ([`crate::window_focus::shared_focus`]).
    pub fn focus_stamp(&self) -> Option<u64> {
        (!self.windows.is_empty()).then(|| self.focus.stamp())
    }

    /// Whether this kind holds the shared focus, so its top window draws
    /// the focused title strip.
    pub fn set_holds_focus(&mut self, holds: bool) {
        self.focus.set_holds(holds);
    }

    /// Whether a window was created since the last call, which the
    /// agent's advice hears of (`FUN_0041d2d0`).
    pub fn take_created(&mut self) -> bool {
        std::mem::take(&mut self.created)
    }

    /// Open `system`'s Missions window at a logical point, clamped into the
    /// galaxy view, with its first mission selected (`FUN_0049f540`,
    /// `FUN_00609500`), or bring its open window to the front: one per
    /// system (`FUN_0045aac0`).
    #[expect(
        clippy::too_many_arguments,
        reason = "Opening selects the first row, which reads the mission and fog state."
    )]
    pub fn open(
        &mut self,
        world: &GameWorld,
        fog: &FogState,
        missions: &MissionState,
        system: SystemKey,
        logical_position: (i16, i16),
        faction: CockpitFaction,
        layout: CockpitLayout,
    ) -> bool {
        self.prepare_faction(faction);
        if !world.systems.contains_key(system) {
            return false;
        }
        if self.focus(system) {
            return true;
        }
        self.created = true;
        if let Some(index) = self.hidden.iter().position(|window| window.system == system) {
            let mut window = self.hidden.remove(index);
            window.moving = None;
            self.windows.push(window);
            self.focus.take();
            return true;
        }
        let player = cockpit_faction(faction);
        let selected = mission_rows(world, fog, missions, player, system)
            .first()
            .map(|row| row.mission);
        self.windows.push(OpenMissionsWindow {
            system,
            logical_position: clamp_window_to_galaxy(
                logical_position,
                layout,
                MISSIONS_WINDOW_WIDTH,
                MISSIONS_WINDOW_HEIGHT,
            ),
            selected,
            tab: MissionsTab::Agents,
            moving: None,
        });
        self.focus.take();
        true
    }

    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    #[must_use]
    pub fn is_open(&self, system: SystemKey) -> bool {
        self.windows.iter().any(|window| window.system == system)
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        let point = egui::pos2(point.0, point.1);
        self.windows
            .iter()
            .any(|window| rect_contains(window_screen_rect(window, layout), point))
    }

    /// The screen rect of the `index`th mission row of `system`'s window.
    #[must_use]
    pub fn mission_row_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        index: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(mission_row_rect(
            window_screen_rect(window, layout),
            layout.scale,
            index,
        ))
    }

    /// The screen rect of `tab`'s button in `system`'s window.
    #[must_use]
    pub fn tab_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        tab: MissionsTab,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(tab_rect(
            window_screen_rect(window, layout),
            layout.scale,
            tab,
        ))
    }

    /// The screen rect of the `index`th member row of `system`'s window.
    #[must_use]
    pub fn member_row_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        index: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(member_row_rect(
            window_screen_rect(window, layout),
            layout.scale,
            index,
        ))
    }

    /// What `system`'s window shows, for the interface fixture.
    #[must_use]
    pub fn report(
        &self,
        world: &GameWorld,
        fog: &FogState,
        missions: &MissionState,
        system: SystemKey,
    ) -> Option<MissionsWindowReport> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        let player = cockpit_faction(self.faction);
        let rows = mission_rows(world, fog, missions, player, system);
        let selected = window
            .selected
            .and_then(|id| rows.iter().position(|row| row.mission == id));
        let selected_row = selected.map(|index| &rows[index]);
        let members = selected_row.map_or_else(Vec::new, |row| {
            member_rows(
                world,
                fog,
                missions,
                player,
                system,
                row.mission,
                window.tab,
            )
        });
        Some(MissionsWindowReport {
            origin: window.logical_position,
            side: missions_side(world, fog, missions, player, system),
            rows: rows
                .iter()
                .map(|row| (row.label.to_string(), row.side, row.picture))
                .collect(),
            selected,
            tab: window.tab,
            tab_side: selected_row.map(|row| row.side),
            member_art: members
                .iter()
                .map(|row| (row.mini, row.marks.iter().map(|&(_, id)| id).collect()))
                .collect(),
            members: members.into_iter().map(|row| row.label).collect(),
            target: selected_row
                .map(|row| target_name(world, target(missions, player, system, row))),
        })
    }

    /// The system a move released over the Missions window egui draws as
    /// `layer` takes (`+0x70`, `FUN_004aa470`: the subject wherever the point
    /// is), or `None` when `layer` is none of them.
    #[must_use]
    pub fn release_target(&self, layer: egui::LayerId) -> Option<SystemKey> {
        self.windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)
            .map(|window| window.system)
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.hidden.clear();
    }

    /// Minimize `system`'s window (`0x466`): hide it, keeping its state, and
    /// return where it stood for its rail slot.
    pub fn hide(&mut self, system: SystemKey) -> Option<(i16, i16)> {
        let window = self.close(system)?;
        let at = window.logical_position;
        self.hidden.push(window);
        Some(at)
    }

    fn prepare_faction(&mut self, faction: CockpitFaction) {
        if self.faction != faction {
            self.faction = faction;
            self.clear();
        }
    }

    /// Show `member` in `system`'s open window (slot 27, `FUN_004a1e10`):
    /// its mission's row is selected and its role bit 0 picks the tab, the
    /// decoys' when set. port: the member row itself is not highlighted.
    pub fn show_member(
        &mut self,
        missions: &MissionState,
        system: SystemKey,
        member: MissionMember,
    ) -> bool {
        let Some(window) = self
            .windows
            .iter_mut()
            .find(|window| window.system == system)
        else {
            return false;
        };
        let Some(mission) = missions
            .missions()
            .iter()
            .find(|mission| mission.team.contains(&member) || mission.decoys.contains(&member))
        else {
            return false;
        };
        window.selected = Some(mission.id);
        window.tab = if mission.decoys.contains(&member) {
            MissionsTab::Decoys
        } else {
            MissionsTab::Agents
        };
        true
    }

    fn focus(&mut self, system: SystemKey) -> bool {
        let Some(index) = self
            .windows
            .iter()
            .position(|window| window.system == system)
        else {
            return false;
        };
        let window = self.windows.remove(index);
        self.windows.push(window);
        self.focus.take();
        true
    }

    fn close(&mut self, system: SystemKey) -> Option<OpenMissionsWindow> {
        let index = self
            .windows
            .iter()
            .position(|window| window.system == system)?;
        Some(self.windows.remove(index))
    }

    fn window_mut(&mut self, system: SystemKey) -> Option<&mut OpenMissionsWindow> {
        self.windows
            .iter_mut()
            .find(|window| window.system == system)
    }
}

/// What one Missions window shows ([`MissionsWindowState::report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionsWindowReport {
    pub origin: (i16, i16),
    /// `FUN_004a1f60`'s side: the title strip and rail icon.
    pub side: u8,
    /// Each mission row's name, side and GOKRES mini.
    pub rows: Vec<(String, u8, u32)>,
    pub selected: Option<usize>,
    pub tab: MissionsTab,
    /// The selected mission's side, which picks the tab art.
    pub tab_side: Option<u8>,
    pub members: Vec<String>,
    /// Each member's GOKRES mini and the marks drawn under it.
    pub member_art: Vec<(Option<u32>, Vec<u32>)>,
    /// The selected mission's target name, or "Target Unknown".
    pub target: Option<String>,
}

/// Actions that leave the Missions window manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionsWindowAction {
    /// The sector button (`0xca`) opens the subject's sector window
    /// (`FUN_00429ce0`).
    OpenSector(SystemKey),
    SelectSystem(SystemKey),
    /// The minimize button (`0x15`) posts `0x466`: the window goes to the
    /// galaxy view's rail.
    Minimize {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A right release on a mission row opens the object pop-up menu
    /// (`FUN_004ac5c0`) for it (the window's slot 22, `FUN_004a0400`, gives
    /// the selected missions; A0 missionstatus/02).
    OpenObjectMenu {
        system: SystemKey,
        mission: u64,
        point: (i16, i16),
    },
}

/// A minimized Missions window's rail icon (`FUN_004a21c0`): 11539 for side
/// 1, 11540 for side 2, 11541 otherwise, by `FUN_004a1f60`'s side.
#[must_use]
pub fn rail_icon(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    faction: CockpitFaction,
    system: SystemKey,
) -> u32 {
    match missions_side(world, fog, missions, cockpit_faction(faction), system) {
        1 => RAIL_ICONS[0],
        2 => RAIL_ICONS[1],
        _ => RAIL_ICONS[2],
    }
}

fn mission(missions: &MissionState, id: u64) -> Option<&ActiveMission> {
    missions.missions().iter().find(|mission| mission.id == id)
}

const fn side_faction(side: u8) -> MissionFaction {
    if side == 1 {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    }
}

/// `FUN_004a1590`'s rows: each distinct mission of the system's members on a
/// visible mission (a mission key, role `+0x78` bit 8 clear), with the side
/// of the first member carrying it. A mission without a MISSNSD class
/// (`FUN_0051cab0`) gets no row.
///
/// port: the player sees the other side's members only where the System
/// window shows them, as for the sector window's icons. hyp: the rows follow
/// the mission keys' order, the map `FUN_004a1590` collects them in, which
/// for the port's sequential ids is the order the missions began.
/// The members the window lists at `system`: those there, then those on
/// their way to it. A member's transit puts it in its target's container
/// at once (`FUN_00556430` to `+0x74`; `mission-lifecycle.md`), so it is
/// listed there, en route (`+0x50` bit 4), as a fleet is
/// (`move-order.md`). Each carries its side, whether it is on a hidden
/// mission, and whether it is en route.
fn listed_members(
    world: &GameWorld,
    fog: &FogState,
    transits: &[rebellion_core::missions::MemberTransit],
    player: Faction,
    system: SystemKey,
) -> Vec<(MissionMember, u8, bool, bool)> {
    let mut members: Vec<_> = visible_members(world, fog, player, system)
        .into_iter()
        .map(|(member, side, hidden)| (member, side, hidden, false))
        .collect();
    // The player's copy's explored bit (ghidra/notes/fog-of-war.md).
    if !rebellion_core::object_views::side_explored(world, player, system) {
        return members;
    }
    let opposing_visible =
        crate::system_window::opposing_contents_visible(world, fog, player, system);
    let own = crate::fleet_window::faction_side(player);
    for (member, side, hidden) in crate::quadrant_icons::transit_members(world, transits, system) {
        if (opposing_visible || side == own) && !members.iter().any(|(m, ..)| *m == member) {
            members.push((member, side, hidden, true));
        }
    }
    members
}

fn mission_rows(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
) -> Vec<MissionRow> {
    let keys = mission_keys(missions);
    let mut sides = BTreeMap::new();
    for (member, side, hidden, _) in listed_members(world, fog, missions.en_route(), player, system)
    {
        if hidden {
            continue;
        }
        if let Some(&key) = keys.get(&member) {
            sides.entry(key).or_insert(side);
        }
    }
    // The other side's missions of the player's frozen copy (FUN_004f2d10,
    // FUN_004f9a60), as seen then, though they may have ended since.
    let copied = copied_missions(world, fog, player, system);
    for copy in &copied {
        sides.entry(copy.id).or_insert(copy.side);
    }
    sides
        .into_iter()
        .filter_map(|(id, side)| {
            let kind = mission(missions, id).map(|value| value.kind).or_else(|| {
                copied.iter().find(|copy| copy.id == id).map(|copy| copy.kind)
            })?;
            kind.record_id()?;
            Some(MissionRow {
                mission: id,
                side,
                picture: MISSION_MINI_BASE + kind_icon(kind, side_faction(side)),
                label: kind_name(kind),
            })
        })
        .collect()
}

/// The other side's missions the player's frozen copy of `system` holds
/// (`SystemDetails::missions`); none while the player sees it live.
fn copied_missions(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Vec<rebellion_core::object_views::MissionDetail> {
    if crate::viewed_contents::frozen_extras(world, fog, player, system).is_none() {
        return Vec::new();
    }
    let own = faction_side(player);
    world
        .systems
        .get(system)
        .and_then(|value| world.object_views.system_details(player, value.dat_id))
        .map(|details| {
            details
                .missions
                .iter()
                .filter(|mission| mission.side != own)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// A copied mission's member rows for `tab`, as the copy holds them: a
/// character by its live object, a special force by its class.
fn copied_member_rows(
    world: &GameWorld,
    copy: &rebellion_core::object_views::MissionDetail,
    tab: MissionsTab,
) -> Vec<MemberRow> {
    use rebellion_core::object_views::MemberCopy;
    let members = if tab == MissionsTab::Decoys {
        &copy.decoys
    } else {
        &copy.team
    };
    members
        .iter()
        .filter_map(|member| match *member {
            MemberCopy::Character(id) => {
                let key = crate::viewed_contents::character_key(world, id)?;
                let character = &world.characters[key];
                Some(MemberRow {
                    member: MissionMember::Character(key),
                    mini: character_picture(character),
                    label: character.name.clone(),
                    marks: Vec::new(),
                })
            }
            MemberCopy::SpecialForce(class) => {
                let (mini, label) = special_force_mini(class)?;
                Some(MemberRow {
                    // port: a copied special force has no live key.
                    member: MissionMember::SpecialForce(Default::default()),
                    mini: Some(mini),
                    label: label.into(),
                    marks: Vec::new(),
                })
            }
        })
        .collect()
}

/// `FUN_004a1ba0`'s rows for `tab`: the members at the system carrying the
/// mission, in the walk's order; the decoys on the Decoys tab, the rest on
/// the Agents tab.
fn member_rows(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
    id: u64,
    tab: MissionsTab,
) -> Vec<MemberRow> {
    // A copied mission lists the members the copy holds (FUN_004f9a60).
    if let Some(copy) = copied_missions(world, fog, player, system)
        .into_iter()
        .find(|copy| copy.id == id)
    {
        return copied_member_rows(world, &copy, tab);
    }
    let Some(value) = mission(missions, id) else {
        return Vec::new();
    };
    let keys = mission_keys(missions);
    listed_members(world, fog, missions.en_route(), player, system)
        .into_iter()
        .filter(|(member, ..)| keys.get(member) == Some(&id))
        .filter(|(member, ..)| value.decoys.contains(member) == (tab == MissionsTab::Decoys))
        .filter_map(|(member, _, _, en_route)| {
            let (mini, label) = member_mini(world, member)?;
            Some(MemberRow {
                member,
                mini,
                label,
                marks: mini
                    .zip(mini_object(world, member))
                    .map(|(mini, object)| {
                        mini_marks(object, mini, en_route, member_injured(world, member))
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// What `member`'s mini stands for, as `FUN_0042c3b0` picks its en route
/// mark.
fn mini_object(world: &GameWorld, member: MissionMember) -> Option<MiniObject> {
    match member {
        MissionMember::Character(_) => Some(MiniObject::Character),
        MissionMember::SpecialForce(key) => world
            .special_forces
            .get(key)
            .map(|force| MiniObject::SpecialForce(force.class_dat_id)),
    }
}

/// A member's GOKRES mini and name (`FUN_0042c3b0(.., 0, 1)`,
/// `FUN_004f62d0`). port: of the mini's status overlays only the en route
/// mark is drawn (`MemberRow::en_route`); the others are not, as in the
/// Mission dialog.
fn member_mini(world: &GameWorld, member: MissionMember) -> Option<(Option<u32>, String)> {
    match member {
        MissionMember::Character(key) => {
            let character = world.characters.get(key)?;
            Some((character_picture(character), character.name.clone()))
        }
        MissionMember::SpecialForce(key) => {
            let force = world.special_forces.get(key)?;
            let (mini, label) = special_force_mini(force.class_dat_id)?;
            Some((Some(mini), label.into()))
        }
    }
}

/// `FUN_004a10a0`'s target `+0x1c8`. The player's own mission names its
/// order's target (`FUN_004f3000`, `+0x70`); another side's mission in
/// families `0x50..0x5f` names the system; any other names nothing.
fn target(
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
    row: &MissionRow,
) -> Option<Target> {
    let value = mission(missions, row.mission)?;
    if row.side == faction_side(player) {
        return Some(if let Some(character) = value.target_character {
            Target::Character(character)
        } else if let Some(object) = value.target_object {
            Target::Object(object)
        } else {
            Target::System(value.target_system)
        });
    }
    let family = value.kind.record_id()?.family();
    (0x50..=0x5f)
        .contains(&family)
        .then_some(Target::System(system))
}

/// The target's picture and name, when it still exists: a system's planet
/// (STRATEGY, `FUN_0045c970`), a character's GOKRES mini (`FUN_0042c3b0(..,
/// 0, 1)`), or another object's GOKRES portrait, its mini less `0x4000`
/// (`FUN_0042c3b0(.., 1, 1)`).
fn target_view(world: &GameWorld, target: Target) -> Option<(DllSource, Option<u32>, String)> {
    let portrait = |found: Option<(u32, &str)>| {
        found.map(|(mini, label)| {
            (
                DllSource::Gokres,
                Some(mini - MISSION_MINI_BASE),
                label.to_string(),
            )
        })
    };
    match target {
        Target::System(key) => {
            let system = world.systems.get(key)?;
            Some((
                DllSource::Strategy,
                Some(planet_resource_id(system.dat_id)),
                system.name.clone(),
            ))
        }
        Target::Character(key) => {
            let character = world.characters.get(key)?;
            Some((
                DllSource::Gokres,
                character_picture(character),
                character.name.clone(),
            ))
        }
        Target::Object(MissionTarget::DefenseFacility(key)) => portrait(defense_facility_mini(
            world.defense_facilities.get(key)?.class_dat_id,
        )),
        Target::Object(MissionTarget::ManufacturingFacility(key)) => portrait(
            manufacturing_facility_mini(world.manufacturing_facilities.get(key)?.class_dat_id),
        ),
        Target::Object(MissionTarget::ProductionFacility(key)) => portrait(
            production_facility_mini(world.production_facilities.get(key)?.class_dat_id),
        ),
        Target::Object(MissionTarget::Troop(key)) => {
            portrait(troop_mini(world.troops.get(key)?.class_dat_id))
        }
        Target::Object(MissionTarget::SpecialForce(key)) => portrait(special_force_mini(
            world.special_forces.get(key)?.class_dat_id,
        )),
        Target::Object(MissionTarget::DeathStar(key)) => {
            let class = world
                .fleets
                .get(key)?
                .capital_ships
                .iter()
                .find_map(|ship| {
                    world
                        .capital_ship_classes
                        .get(ship.class)
                        .filter(|class| ship.alive && class.is_death_star())
                })?;
            Some((
                DllSource::Gokres,
                capital_ship_mini_id(class.dat_id).map(|mini| mini - MISSION_MINI_BASE),
                class.name.clone(),
            ))
        }
        Target::Object(MissionTarget::CapitalShip { class, .. }) => {
            let class = world.capital_ship_classes.get(class)?;
            Some((
                DllSource::Gokres,
                capital_ship_mini_id(class.dat_id).map(|mini| mini - MISSION_MINI_BASE),
                class.name.clone(),
            ))
        }
        Target::Object(MissionTarget::Fighter { class, .. }) => {
            let class = world.fighter_classes.get(class)?;
            Some((
                DllSource::Gokres,
                crate::minis::fighter_mini_id(class.dat_id)
                    .map(|mini| mini - MISSION_MINI_BASE),
                class.name.clone(),
            ))
        }
    }
}

/// `+0x174`: the target's name, or TEXTSTRA 34087 "Target Unknown".
fn target_name(world: &GameWorld, target: Option<Target>) -> String {
    target
        .and_then(|target| target_view(world, target))
        .map_or_else(|| "Target Unknown".to_string(), |(_, _, name)| name)
}

/// Where a bitmap lands centred in a box: the original's integer
/// `(box - size) / 2` offsets (`FUN_0049fef0`, `FUN_004a0e10`).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Boxes and bitmap sizes are small integers."
)]
fn centered(box_size: (f32, f32), size: [usize; 2]) -> (f32, f32) {
    let offset = |outer: f32, inner: usize| ((outer as i32 - inner as i32) / 2) as f32;
    (offset(box_size.0, size[0]), offset(box_size.1, size[1]))
}

/// Where the target picture lands: centred in its box (`FUN_0049fef0`), or
/// at the box's corner when its size is unknown.
fn picture_at(size: Option<[usize; 2]>) -> (f32, f32) {
    let (x, y) = size.map_or((0.0, 0.0), |size| centered((PICTURE.2, PICTURE.3), size));
    (PICTURE.0 + x, PICTURE.1 + y)
}

/// A member row's text top (`FUN_004a0e10`, item `+0x34`): the mini's height
/// plus its centring offset, so the name sits under the mini.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "Bitmap heights are small integers."
)]
fn member_text_top(mini_height: usize) -> f32 {
    let height = mini_height as i32;
    (height + (MEMBER_IMAGE.1 as i32 - height) / 2) as f32
}

fn cockpit_faction(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn area_id(system: SystemKey) -> egui::Id {
    egui::Id::new(("original-missions-window", system))
}

fn window_screen_rect(window: &OpenMissionsWindow, layout: CockpitLayout) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + f32::from(window.logical_position.0) * layout.scale,
            layout.canvas.y + f32::from(window.logical_position.1) * layout.scale,
        ),
        egui::vec2(
            MISSIONS_WINDOW_WIDTH * layout.scale,
            MISSIONS_WINDOW_HEIGHT * layout.scale,
        ),
    )
}

/// What `tab`'s key pixel, its art's bottom-left corner, shows: the window's
/// background 11165 at the tab's position inside the strip, not inside the
/// window (`FUN_00602d30`; see the paint call).
fn tab_key_pixel(tab: MissionsTab) -> KeyPixel {
    // The tab's x in the strip (MissionsTab::x) and the strip's last row.
    let x = match tab {
        MissionsTab::Agents => 0,
        MissionsTab::Decoys => 61,
    };
    KeyPixel::BackgroundAt(BACKGROUND, x, 15)
}

fn tab_rect(window: egui::Rect, scale: f32, tab: MissionsTab) -> egui::Rect {
    logical_rect(
        window,
        scale,
        TAB_STRIP.0 + tab.x(),
        TAB_STRIP.1,
        TAB_SIZE.0,
        TAB_SIZE.1,
    )
}

/// The `index`th mission row (`FUN_00609ae0`, `+0xf0` 1): stacked from the
/// list's corner.
#[expect(clippy::cast_precision_loss, reason = "Row indexes are small.")]
fn mission_row_rect(window: egui::Rect, scale: f32, index: usize) -> egui::Rect {
    logical_rect(
        window,
        scale,
        MISSION_LIST.0,
        MISSION_LIST.1 + index as f32 * MISSION_ROW.1,
        MISSION_ROW.0,
        MISSION_ROW.1,
    )
}

/// The `index`th member row (`FUN_00609ae0`'s grid): the next cell would
/// pass the list's width, so each row holds one.
#[expect(clippy::cast_precision_loss, reason = "Row indexes are small.")]
fn member_row_rect(window: egui::Rect, scale: f32, index: usize) -> egui::Rect {
    logical_rect(
        window,
        scale,
        MEMBER_LIST.0,
        MEMBER_LIST.1 + index as f32 * MEMBER_ROW.1,
        MEMBER_ROW.0,
        MEMBER_ROW.1,
    )
}

#[derive(Default)]
struct WindowDrawResult {
    focus: bool,
    close: bool,
    minimize: bool,
    open_sector: bool,
    select: Option<u64>,
    tab: Option<MissionsTab>,
    /// The pointer's travel since a press on the window's surface.
    travel: Option<egui::Vec2>,
    /// A right release on a mission row: the row's mission, selected first,
    /// and the 640 by 480 canvas point.
    object_menu: Option<(u64, (i16, i16))>,
}

/// Draw every open Missions window.
///
/// port: the scroll bars, the tab help text, the list drags (`FUN_004a0400`)
/// and Escape (`FUN_004a0e00`) are not ported; the galaxy view's windows keep
/// no shared keyboard focus to route a key by.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and rendering inputs at this UI boundary, as the Fleet window does."
)]
pub fn draw_missions_windows(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    state: &mut MissionsWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Vec<MissionsWindowAction> {
    state.prepare_faction(faction);
    let player = cockpit_faction(faction);
    // FUN_004a0c20: a selected mission that leaves the list clears the
    // selection.
    for window in &mut state.windows {
        let rows = mission_rows(world, fog, missions, player, window.system);
        if window
            .selected
            .is_some_and(|id| !rows.iter().any(|row| row.mission == id))
        {
            window.selected = None;
        }
    }
    let windows = state.windows.clone();
    let focused_system = windows
        .last()
        .filter(|_| state.focus.holds())
        .map(|window| window.system);
    let mut actions = Vec::new();
    for window in &windows {
        let mut result = draw_missions_window(
            ctx,
            world,
            fog,
            missions,
            window,
            focused_system == Some(window.system),
            faction,
            layout,
            cache,
        );
        let system = window.system;
        if result.close || !world.systems.contains_key(system) {
            state.close(system);
            continue;
        }
        if result.minimize {
            if let Some(logical_position) = state.hide(system) {
                actions.push(MissionsWindowAction::Minimize {
                    system,
                    logical_position,
                });
            }
            continue;
        }
        if result.open_sector {
            actions.push(MissionsWindowAction::OpenSector(system));
        }
        if let Some(open) = state.window_mut(system) {
            if result.travel.is_some() && open.moving.is_none() {
                result.focus = true;
            }
            window_move::follow(
                &mut open.logical_position,
                &mut open.moving,
                result.travel,
                |position| {
                    clamp_window_to_galaxy(
                        position,
                        layout,
                        MISSIONS_WINDOW_WIDTH,
                        MISSIONS_WINDOW_HEIGHT,
                    )
                },
            );
            // FUN_004a0c60: a different mission refreshes the details and
            // reselects the Agents tab (FUN_004a0ca0).
            if let Some(id) = result.select.filter(|&id| open.selected != Some(id)) {
                open.selected = Some(id);
                open.tab = MissionsTab::Agents;
            }
            if let Some(tab) = result.tab {
                open.tab = tab;
            }
        }
        if result.focus {
            state.focus(system);
            actions.push(MissionsWindowAction::SelectSystem(system));
        }
        if let Some((mission, point)) = result.object_menu {
            actions.push(MissionsWindowAction::OpenObjectMenu {
                system,
                mission,
                point,
            });
        }
    }
    actions
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Keep the window's ordered paint and input pass together, as the Defenses window does."
)]
fn draw_missions_window(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    window: &OpenMissionsWindow,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> WindowDrawResult {
    let mut result = WindowDrawResult::default();
    let Some(system) = world.systems.get(window.system) else {
        result.close = true;
        return result;
    };
    let player = cockpit_faction(faction);
    let side = missions_side(world, fog, missions, player, window.system);
    let rows = mission_rows(world, fog, missions, player, window.system);
    let selected = window
        .selected
        .and_then(|id| rows.iter().find(|row| row.mission == id));
    let scale = layout.scale;
    let screen_rect = window_screen_rect(window, layout);
    let id = area_id(window.system);
    if focused {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, id));
    }
    let area = egui::Area::new(id)
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            let (local, window_response) =
                ui.allocate_exact_size(screen_rect.size(), egui::Sense::click_and_drag());
            let painter = ui.painter().clone();
            let white = egui::Color32::WHITE;
            // The window's labels are font 5 (16 rows, bold); the mission
            // list writes in font 10 (FUN_0049f540: its slot +0x18 with 10)
            // and the member rows' names are drawn in font 10 into their
            // images (FUN_004a1ba0). DrawTextA sets each
            // (theme::paint_gdi_text).
            use crate::theme::{paint_gdi_text, paint_gdi_text_block, GdiAlign, GdiVAlign};

            painter.rect_filled(local, 0.0, egui::Color32::BLACK);
            paint_native(
                &painter,
                ctx,
                cache,
                DllSource::Strategy,
                BACKGROUND,
                local,
                scale,
                0.0,
                0.0,
            );

            // The paint slot (FUN_0049fef0): the target picture centred in
            // its box, `+0x174` and `+0x178`, then the strip and the title.
            if let Some(row) = selected {
                let target = target(missions, player, window.system, row);
                let view = target.and_then(|target| target_view(world, target));
                if let Some((source, Some(picture), _)) = &view {
                    let (x, y) = picture_at(cache.original_resource_size(*source, *picture));
                    paint_native(&painter, ctx, cache, *source, *picture, local, scale, x, y);
                }
                // `+0x174`: font 5, format 0x11 (centred, word-wrapped).
                let name = view.map_or_else(|| "Target Unknown".to_string(), |(_, _, name)| name);
                let _ = paint_gdi_text_block(
                    &painter,
                    logical_rect(
                        local,
                        scale,
                        TARGET_NAME.0,
                        TARGET_NAME.1,
                        TARGET_NAME.2,
                        32.0,
                    ),
                    scale,
                    5,
                    &name,
                    white,
                    GdiAlign::Center,
                );
                // `+0x178`: font 5, format 1 (centred), TEXTSTRA 34081.
                paint_gdi_text(
                    &painter,
                    logical_rect(
                        local,
                        scale,
                        TARGET_LABEL.0,
                        TARGET_LABEL.1,
                        TARGET_LABEL.2,
                        16.0,
                    ),
                    scale,
                    5,
                    "Target:",
                    white,
                    (GdiAlign::Center, GdiVAlign::Top),
                );
            }
            paint_native(
                &painter,
                ctx,
                cache,
                DllSource::Strategy,
                title_resource(side, focused),
                local,
                scale,
                2.0,
                2.0,
            );
            // `+0x17c`: TEXTSTRA 34085 and the system name, font 5, format
            // 0x24 (one line, left-aligned, vertically centred).
            paint_gdi_text(
                &painter,
                logical_rect(local, scale, TITLE.0, TITLE.1, TITLE.2, TITLE.3),
                scale,
                5,
                &format!("Mission at {}", system.name),
                egui::Color32::BLACK,
                (GdiAlign::Left, GdiVAlign::VCenter),
            );

            let buttons = [
                (SECTOR_NORMAL, SECTOR_PRESSED, 3.0, "sector"),
                (MINIMIZE_NORMAL, MINIMIZE_PRESSED, 204.0, "minimize"),
                (CLOSE_NORMAL, CLOSE_PRESSED, 218.0, "close"),
            ];
            let mut clicked = [false; 3];
            for (index, (normal, pressed, x, name)) in buttons.into_iter().enumerate() {
                let rect = logical_rect(local, scale, x, 3.0, 14.0, 14.0);
                let response = ui.interact(
                    rect,
                    ui.id().with((window.system, name)),
                    egui::Sense::click(),
                );
                let down = primary_down && pointer.is_some_and(|point| rect_contains(rect, point));
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    if down { pressed } else { normal },
                    local,
                    scale,
                    x,
                    3.0,
                );
                // The buttons are children blitted keyed over the window
                // (FUN_0049f540 makes them with FUN_00602150; FUN_00602d30
                // -> FUN_005fd0f0): A0 mission-reports/missions-window-
                // enroute-day276 shows 11165's grey (90, 88, 94) at (3, 16),
                // (204, 16) and (218, 16), not the key's green or the strip.
                paint_key_pixel(
                    &painter,
                    ctx,
                    cache,
                    local,
                    scale,
                    x,
                    16.0,
                    KeyPixel::Background(BACKGROUND),
                );
                clicked[index] = exact_clicked(&response, rect);
            }
            result.open_sector = clicked[0];
            result.minimize = clicked[1];
            result.close = clicked[2];

            // The mission list: each picture keyed at its row's corner, the
            // frame keyed over it while selected, and the name word-wrapped
            // from (1, 0), white either way (`FUN_00607ea0`'s `+0xdc`).
            // port: the scroll bar is not drawn, so the rows past the
            // list's foot are cut off.
            let list = logical_rect(
                local,
                scale,
                MISSION_LIST.0,
                MISSION_LIST.1,
                MISSION_LIST.2,
                MISSION_LIST.3,
            );
            let list_painter = painter.with_clip_rect(list);
            let frame = if faction_side(player) == 1 {
                MISSION_FRAME[0]
            } else {
                MISSION_FRAME[1]
            };
            for (index, row) in rows.iter().enumerate() {
                let cell = mission_row_rect(local, scale, index);
                if !cell.intersects(list) {
                    break;
                }
                paint_native(
                    &list_painter,
                    ctx,
                    cache,
                    DllSource::Gokres,
                    row.picture,
                    cell,
                    scale,
                    0.0,
                    0.0,
                );
                if window.selected == Some(row.mission) {
                    paint_native(
                        &list_painter,
                        ctx,
                        cache,
                        DllSource::Strategy,
                        frame,
                        cell,
                        scale,
                        0.0,
                        0.0,
                    );
                }
                let _ = paint_gdi_text_block(
                    &list_painter,
                    logical_rect(
                        cell,
                        scale,
                        TEXT_OFFSET.0,
                        TEXT_OFFSET.1,
                        MISSION_ROW.0 - TEXT_RIGHT_TRIM,
                        MISSION_ROW.1 - TEXT_OFFSET.1,
                    ),
                    scale,
                    10,
                    row.label,
                    white,
                    GdiAlign::Left,
                );
                let response = ui.interact(
                    cell.intersect(list),
                    ui.id().with((window.system, "mission", index)),
                    egui::Sense::click(),
                );
                if exact_clicked(&response, cell) {
                    result.select = Some(row.mission);
                    result.focus = true;
                }
                if response.secondary_clicked() {
                    if let Some(point) = response
                        .interact_pointer_pos()
                        .filter(|point| rect_contains(cell.intersect(list), *point))
                    {
                        result.select = Some(row.mission);
                        result.focus = true;
                        result.object_menu = Some((
                            row.mission,
                            crate::system_window::canvas_point(layout, point),
                        ));
                    }
                }
            }

            // With a mission selected, its side's tabs and the current
            // tab's members (FUN_004a0ca0); with none, both hide.
            if let Some(row) = selected {
                for tab in MissionsTab::ALL {
                    let rect = tab_rect(local, scale, tab);
                    if let Some(art) = tab_resource(tab, row.side, tab == window.tab) {
                        paint_native(
                            &painter,
                            ctx,
                            cache,
                            DllSource::Strategy,
                            art,
                            rect,
                            scale,
                            0.0,
                            0.0,
                        );
                        // The tab is a CoolStrobeButton in the strip,
                        // keyed on its first DIB pixel over the window's
                        // surface 11165 read at its strip position
                        // (FUN_0049f540 hands FUN_00602150 the window's
                        // +0xa4; FUN_00602d30 copies it from (+0x10c,
                        // +0x110), the tab's (x, 0) in the strip): A0
                        // mission-reports/missions-window-enroute-day276
                        // shows 11165's (0, 15) and (61, 15).
                        paint_key_pixel(
                            &painter,
                            ctx,
                            cache,
                            local,
                            scale,
                            TAB_STRIP.0 + tab.x(),
                            TAB_STRIP.1 + TAB_SIZE.1 - 1.0,
                            tab_key_pixel(tab),
                        );
                    }
                    let response = ui.interact(
                        rect,
                        ui.id().with((window.system, tab)),
                        egui::Sense::click(),
                    );
                    crate::win_tooltip::response_tool(&response, rect, tab.tooltip_string());
                    if exact_clicked(&response, rect) {
                        result.tab = Some(tab);
                        result.focus = true;
                    }
                }
                let members = logical_rect(
                    local,
                    scale,
                    MEMBER_LIST.0,
                    MEMBER_LIST.1,
                    MEMBER_LIST.2,
                    MEMBER_LIST.3,
                );
                let member_painter = painter.with_clip_rect(members);
                let rows = member_rows(
                    world,
                    fog,
                    missions,
                    player,
                    window.system,
                    row.mission,
                    window.tab,
                );
                for (index, member) in rows.iter().enumerate() {
                    let cell = member_row_rect(local, scale, index);
                    if !cell.intersects(members) {
                        break;
                    }
                    let size = member
                        .mini
                        .and_then(|mini| cache.original_resource_size(DllSource::Gokres, mini));
                    if let Some(mini) = member.mini {
                        let (x, y) = size.map_or((0.0, 0.0), |size| centered(MEMBER_IMAGE, size));
                        // FUN_0042c3b0: the mark under the mini, which is
                        // blitted keyed over it (`fleet-window.md`).
                        for (source, id) in member
                            .marks
                            .iter()
                            .copied()
                            .chain(std::iter::once((DllSource::Gokres, mini)))
                        {
                            paint_native(
                                &member_painter,
                                ctx,
                                cache,
                                source,
                                id,
                                cell,
                                scale,
                                x,
                                y,
                            );
                        }
                    }
                    // Format 0x21: one line, centred in the cell less 2 on
                    // each side, under the mini (item `+0x34`), font 10.
                    paint_gdi_text(
                        &member_painter,
                        logical_rect(
                            cell,
                            scale,
                            TEXT_OFFSET.0,
                            member_text_top(size.map_or(0, |size| size[1])),
                            MEMBER_ROW.0,
                            16.0,
                        ),
                        scale,
                        10,
                        &member.label,
                        white,
                        (GdiAlign::Center, GdiVAlign::Top),
                    );
                    let response = ui.interact(
                        cell.intersect(members),
                        ui.id().with((window.system, "member", index)),
                        egui::Sense::click(),
                    );
                    if exact_clicked(&response, cell) {
                        result.focus = true;
                    }
                }
            }

            if window_response.clicked() || clicked.iter().any(|&value| value) {
                result.focus = true;
            }
            // The child controls: the title buttons, the mission list and,
            // with a mission selected, the tabs and the member list.
            let mut controls: Vec<egui::Rect> = [3.0, 204.0, 218.0]
                .into_iter()
                .map(|x| logical_rect(local, scale, x, 3.0, 14.0, 14.0))
                .collect();
            controls.push(logical_rect(
                local,
                scale,
                MISSION_LIST.0,
                MISSION_LIST.1,
                MISSION_LIST.2,
                MISSION_LIST.3,
            ));
            if window.selected.is_some() {
                controls.extend(MissionsTab::ALL.map(|tab| tab_rect(local, scale, tab)));
                controls.push(logical_rect(
                    local,
                    scale,
                    MEMBER_LIST.0,
                    MEMBER_LIST.1,
                    MEMBER_LIST.2,
                    MEMBER_LIST.3,
                ));
            }
            result.travel =
                window_move::held_travel(ui, &window_response, &controls, window.moving, scale);
        });
    if area.response.clicked() {
        result.focus = true;
    }
    result
}

#[cfg(test)]
#[path = "missions_window_tests.rs"]
mod tests;
