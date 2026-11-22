//! Original strategic sector windows.
//!
//! `REBEXE.EXE` constructs these as modeless 235 by 360 child windows. The
//! window body is data-driven: each system uses its `SYSTEMSD.DAT` planet
//! picture, sector-relative coordinates, and three compact status tracks.
//! The close and side-switch controls use their original STRATEGY resources.
//! A system with fleets shows the fleet icon at its planet's top right; a
//! double click on it opens the Fleet window (`ghidra/notes/fleet-window.md`).

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{DatId, SectorKey, SystemKey};
use rebellion_core::pictures::planet_picture_id;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::object_menu::MenuObject;
#[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
use crate::panels::command_palette::InterfaceCommand;
use crate::quadrant_icons::{quadrant_icon, Quadrant};

/// `DAT_00658bd8`, a static 1023: the galaxy's width, whose half
/// `FUN_00429ce0` compares a sector's x with (`FUN_00526560`, the record's
/// `+0x48`). hyp: that x is `Sector.x`, SECTORSD's map position.
const GALAXY_WIDTH: i32 = 1023;

pub const SECTOR_WINDOW_WIDTH: f32 = 235.0;
pub const SECTOR_WINDOW_HEIGHT: f32 = 360.0;

const CLOSE_NORMAL: u32 = 10108;
const CLOSE_PRESSED: u32 = 10109;
const SWITCH_SIDE_NORMAL: u32 = 10210;
const SWITCH_SIDE_PRESSED: u32 = 10211;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowColumn {
    Primary,
    Secondary,
}

impl WindowColumn {
    fn opposite(self) -> Self {
        match self {
            Self::Primary => Self::Secondary,
            Self::Secondary => Self::Primary,
        }
    }
}

/// A selected overlay and the art it showed when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedIcon {
    system: SystemKey,
    /// A quadrant icon, or `None` for the headquarters overlay (id
    /// `0x10000`, kind 1).
    quadrant: Option<Quadrant>,
    /// The icon's first bitmap when it was chosen, which tells a changed
    /// icon from the one chosen.
    art: u32,
    /// What lay at its system when it was chosen ([`system_contents`]).
    contents: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenSectorWindow {
    sector: SectorKey,
    column: WindowColumn,
    /// The icon a left or right press chose last (`+0x184`,
    /// `FUN_0045b1b0`).
    selection: Option<SelectedIcon>,
    /// Where a left press on an icon began a drag, as a canvas point
    /// (`+0x14c`/`+0x150`, set by `FUN_004593e0` case `0x201`).
    drag_from: Option<(i16, i16)>,
}

/// Mutable state for the modeless sector-window stack. The last entry is the
/// focused window and therefore paints above earlier entries.
#[derive(Debug)]
pub struct SectorWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenSectorWindow>,
    /// The galaxy view's starfield (STRATEGY 902 or 903), which the windows
    /// show shaded under themselves.
    backdrop: u32,
    /// Each system's GID marker bitmap in the galaxy view this frame.
    gid_markers: std::collections::HashMap<SystemKey, u32>,
    /// The markers the galaxy view drew and their screen centres.
    drawn_gid_markers: Vec<(u32, [f32; 2])>,
    /// The caption the galaxy view drew (`FUN_00425d00`), which the
    /// windows' copy of the view shows shaded.
    gid_caption: Option<crate::GidCaption>,
    /// A window was created, not raised, since [`Self::take_created`]
    /// (`FUN_00429ce0` then calls `FUN_0041d2d0`).
    created: bool,
    /// The drag image the last pass painted ([`Self::drag_image`]).
    drag_image: Option<(u32, egui::Rect)>,
    /// The headquarters overlays the last pass painted: each window's area,
    /// the system and the mark's planet rect ([`Self::headquarters_at`]).
    headquarters_marks: Vec<(egui::Id, SystemKey, egui::Rect)>,
}

impl Default for SectorWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            backdrop: crate::bmp_cache::resources::strategy::GALAXY_STARFIELD_DIM,
            gid_markers: std::collections::HashMap::new(),
            drawn_gid_markers: Vec::new(),
            gid_caption: None,
            created: false,
            drag_image: None,
            headquarters_marks: Vec::new(),
        }
    }
}

impl SectorWindowState {
    /// Whether a window was created since the last call, which the
    /// agent's advice hears of (`FUN_0041d2d0`).
    pub fn take_created(&mut self) -> bool {
        std::mem::take(&mut self.created)
    }

    /// The image a held icon drag painted in the last pass: the dragged
    /// icon's bitmap and its screen rect moved with the pointer
    /// (`FUN_004593e0` case `0x200`).
    #[must_use]
    pub fn drag_image(&self) -> Option<(u32, egui::Rect)> {
        self.drag_image
    }

    /// Show `gid_mode`'s starfield under the windows, the one the galaxy view
    /// paints (`draw_galaxy_backdrop`).
    pub fn set_backdrop(&mut self, gid_mode: crate::cockpit::GidMode) {
        self.backdrop = crate::gid_backdrop_resource(gid_mode);
    }

    /// Take the galaxy view's GID markers (`GalaxyMapState::gid_markers`),
    /// which each planet shows at its bottom left (`FUN_0042d860`).
    pub fn set_gid_markers(&mut self, markers: &std::collections::HashMap<SystemKey, u32>) {
        self.gid_markers.clone_from(markers);
    }

    /// Take the markers the galaxy view drew, which the windows' copy of the
    /// view shows shaded (`GalaxyMapState::drawn_gid_markers`).
    pub fn set_drawn_gid_markers(&mut self, markers: &[(u32, [f32; 2])]) {
        self.drawn_gid_markers = markers.to_vec();
    }

    /// Take the caption the galaxy view drew ([`crate::gid_caption`]),
    /// which the windows' copy of the view shows shaded (A0
    /// sector-windows/02-second: "Popular Support" under both windows).
    pub fn set_gid_caption(&mut self, caption: Option<crate::GidCaption>) {
        self.gid_caption = caption;
    }

    /// Open the selected system's parent sector (`FUN_00429ce0`;
    /// `ghidra/notes/sector-window-placement.md`). An open window raises
    /// rather than duplicate. At most two are open, one per column: a sector
    /// on the galaxy's right half takes the second column when both are
    /// free, the free column otherwise, and with both taken it replaces the
    /// window in its own half's column.
    pub fn open_for_system(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        faction: CockpitFaction,
    ) -> bool {
        self.prepare_faction(faction);
        let Some(sector) = world.systems.get(system).map(|system| system.sector) else {
            return false;
        };
        if self.focus(sector) {
            return true;
        }
        let right_half = world
            .sectors
            .get(sector)
            .is_some_and(|value| i32::from(value.x) >= GALAXY_WIDTH / 2);
        let half = if right_half {
            WindowColumn::Secondary
        } else {
            WindowColumn::Primary
        };
        let taken = |column| self.windows.iter().any(|window| window.column == column);
        let column = match (taken(WindowColumn::Primary), taken(WindowColumn::Secondary)) {
            (false, false) | (true, true) => half,
            (true, false) => WindowColumn::Secondary,
            (false, true) => WindowColumn::Primary,
        };
        // FUN_00600f90 destroys the window the new one replaces.
        self.windows.retain(|window| window.column != column);
        self.created = true;
        self.windows.push(OpenSectorWindow {
            sector,
            column,
            selection: None,
            drag_from: None,
        });
        true
    }

    /// What a palette command asks of the sector windows. It opens the
    /// sector window holding its system, then gives the action a double
    /// click (`FUN_004593e0` case `0x203`) or a right click (cases `0x204`
    /// and `0x205`) on the system's icon would, at the icon's center. Every
    /// quadrant needs its icon shown, the System window too: case `0x203`
    /// hit-tests the icons only, so the planet opens nothing. `Err` says
    /// why the command did nothing.
    #[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
    #[expect(
        clippy::too_many_arguments,
        reason = "The command reads the same world views the window paints from."
    )]
    pub fn command_actions(
        &mut self,
        world: &GameWorld,
        fog: &FogState,
        missions: &rebellion_core::missions::MissionState,
        movement: &rebellion_core::movement::MovementState,
        travellers: crate::quadrant_icons::Travellers<'_>,
        faction: CockpitFaction,
        layout: CockpitLayout,
        command: InterfaceCommand,
    ) -> Result<Vec<SectorWindowAction>, &'static str> {
        let (system, quadrant, menu) = match command {
            InterfaceCommand::StartGame(_) => return Err("the app starts a game"),
            InterfaceCommand::OpenSector(system) => {
                return if self.open_for_system(world, system, faction) {
                    Ok(Vec::new())
                } else {
                    Err("no such system")
                };
            }
            InterfaceCommand::OpenQuadrantWindow { system, quadrant } => (system, quadrant, false),
            InterfaceCommand::OpenIconMenu { system, quadrant } => (system, quadrant, true),
        };
        if !self.open_for_system(world, system, faction) {
            return Err("no such system");
        }
        let point = self
            .quadrant_window_point(world, layout, system, quadrant)
            .ok_or("no such system")?;
        let shown = quadrant_icon(
            world,
            fog,
            missions,
            movement,
            travellers,
            cockpit_faction(faction),
            system,
            quadrant,
        );
        if menu {
            let (art, _) = shown.ok_or("the icon is hidden")?;
            // As a press on it does (FUN_0045b1b0).
            let sector = world.systems[system].sector;
            if let Some(window) = self
                .windows
                .iter_mut()
                .find(|window| window.sector == sector)
            {
                window.selection = Some(SelectedIcon {
                    system,
                    quadrant: Some(quadrant),
                    art,
                    contents: system_contents(world, system),
                });
            }
            return Ok(vec![SectorWindowAction::OpenObjectMenu {
                selection: Some(MenuObject::SystemIcon { system, quadrant }),
                point,
            }]);
        }
        if shown.is_none() {
            return Err("the icon is hidden");
        }
        Ok(vec![quadrant_window_action(system, quadrant, point)])
    }

    /// True when the pointer lies inside any visible window. Logical right and
    /// bottom edges remain exclusive, as in the recovered Win32 rectangles.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.windows.iter().any(|window| {
            let rect = window_screen_rect(self.faction, window.column, layout);
            point.0 >= rect.min.x
                && point.0 < rect.max.x
                && point.1 >= rect.min.y
                && point.1 < rect.max.y
        })
    }

    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Where `system`'s planet is drawn, in screen pixels, when its sector's
    /// window is open.
    #[must_use]
    pub fn planet_screen_rect(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        system: SystemKey,
    ) -> Option<egui::Rect> {
        let value = world.systems.get(system)?;
        let window = self
            .windows
            .iter()
            .find(|window| window.sector == value.sector)?;
        let sector = world.sectors.get(window.sector)?;
        let (x, y) = sector_planet_position(sector.x, sector.y, value.x, value.y);
        let window_rect = window_screen_rect(self.faction, window.column, layout);
        Some(planet_rect(window_rect, layout.scale, x, y))
    }

    /// Where `system`'s top-right quadrant overlay lies, in screen pixels,
    /// when its sector's window is open: 28 by 19 from one pixel right of
    /// the planet's center to its center row (`FUN_00459e30`, flag
    /// `0x100000`).
    #[must_use]
    pub fn fleet_icon_screen_rect(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        system: SystemKey,
    ) -> Option<egui::Rect> {
        self.quadrant_screen_rect(world, layout, system, Quadrant::Fleets)
    }

    /// The logical point `system`'s Fleet window opens at from its open
    /// sector window: the fleet icon's top left (`FUN_0045c8e0` →
    /// `FUN_0045aac0(.., item +0x40, +0x44)`).
    #[must_use]
    pub fn fleet_window_point(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        system: SystemKey,
    ) -> Option<(i16, i16)> {
        self.quadrant_window_point(world, layout, system, Quadrant::Fleets)
    }

    /// The logical point the window behind one of `system`'s quadrant icons
    /// opens at when a Finder or the rail opens it (`FUN_0045c8e0` passes the
    /// item's `+0x40`, `+0x44`, its rect's top left, to `FUN_0045aac0`):
    /// A0 personnel-states/17, Pzob's Defenses window from the Troop Finder
    /// at x 118, its icon's left edge. A double click opens at the pointer
    /// instead (`FUN_004593e0`).
    #[must_use]
    pub fn quadrant_window_point(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        system: SystemKey,
        quadrant: Quadrant,
    ) -> Option<(i16, i16)> {
        let icon = self.quadrant_screen_rect(world, layout, system, quadrant)?;
        Some(screen_to_logical(layout, icon.min))
    }

    /// Where one of `system`'s four quadrant overlays lies, in screen
    /// pixels, when its sector's window is open (`FUN_00459e30`).
    #[must_use]
    pub fn quadrant_screen_rect(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        system: SystemKey,
        quadrant: Quadrant,
    ) -> Option<egui::Rect> {
        let planet = self.planet_screen_rect(world, layout, system)?;
        Some(quadrant_rect(planet, layout.scale, quadrant))
    }

    /// The destination a targeting release at `point` takes from the sector
    /// window egui draws as `layer`, or `None` when `layer` is none of them.
    ///
    /// The window's `+0x70` (`FUN_0045c830` → `FUN_0045c660`) gives the first
    /// planet whose rectangle holds the point, and no system between planets.
    /// `FUN_00459e30` sets each rectangle once to the 37 by 37 planet bitmap
    /// at its position (`ghidra/notes/sector-window-hit-test.md`). hyp: the
    /// items are listed in the sector's system order.
    #[must_use]
    pub fn release_target(
        &self,
        world: &GameWorld,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<Option<SystemKey>> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.sector) == layer.id)?;
        let Some(sector) = world.sectors.get(window.sector) else {
            return Some(None);
        };
        let window_rect = window_screen_rect(self.faction, window.column, layout);
        Some(sector.systems.iter().copied().find(|key| {
            world.systems.get(*key).is_some_and(|system| {
                let (x, y) = sector_planet_position(sector.x, sector.y, system.x, system.y);
                rect_contains(planet_rect(window_rect, layout.scale, x, y), point)
            })
        }))
    }

    /// The system whose headquarters overlay the window on `layer` showed
    /// under `point` in the last pass. `FUN_0045cc10` walks the overlay list
    /// first, and the overlay (id `0x10000`, `FUN_0045bbb0`) takes a point
    /// wherever its bitmap 0x388 is opaque, so the window's `+0x68` names the
    /// headquarters object there, not the planet.
    pub fn headquarters_at(
        &self,
        cache: &mut BmpCache,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<SystemKey> {
        self.headquarters_marks
            .iter()
            .find(|&&(id, _, rect)| {
                id == layer.id && headquarters_hit(cache, rect, layout.scale, point)
            })
            .map(|&(_, system, _)| system)
    }

    pub fn clear(&mut self) {
        self.windows.clear();
    }

    fn prepare_faction(&mut self, faction: CockpitFaction) {
        if self.faction != faction {
            self.faction = faction;
            self.windows.clear();
        }
    }

    fn focus(&mut self, sector: SectorKey) -> bool {
        let Some(index) = self
            .windows
            .iter()
            .position(|window| window.sector == sector)
        else {
            return false;
        };
        let window = self.windows.remove(index);
        self.windows.push(window);
        true
    }

    fn close(&mut self, sector: SectorKey) {
        self.windows.retain(|window| window.sector != sector);
    }

    fn switch_side(&mut self, sector: SectorKey) {
        if let Some(window) = self
            .windows
            .iter_mut()
            .find(|window| window.sector == sector)
        {
            window.column = window.column.opposite();
        }
        self.focus(sector);
    }
}

/// Actions that leave the sector-window manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorWindowAction {
    SelectSystem(SystemKey),
    OpenSystemWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A double click on a shown fleet icon (`FUN_004593e0` case `0x203` →
    /// `FUN_0045aac0`, kind `0x10`).
    OpenFleetWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A double click on a shown defenses icon (`FUN_0045aac0`, kind 8):
    /// the System Defenses window, type 10.
    OpenDefensesWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A double click on a shown missions icon (`FUN_0045aac0`, kind
    /// `0x40`): the Missions window, type 11.
    OpenMissionsWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A right-button release opens the object pop-up menu (slot 7,
    /// `FUN_004ac5c0`) for the window's selection, at a 640 by 480 canvas
    /// point.
    OpenObjectMenu {
        selection: Option<MenuObject>,
        point: (i16, i16),
    },
    /// A left release more than four pixels from the press that chose an
    /// icon (`FUN_004593e0` case `0x202`) posts `0x29a`: the galaxy view
    /// moves the selection to what lies under the screen `point`, as a
    /// Confirmed Move when Ctrl is held (`FUN_00422ce0`).
    DropSelection {
        selection: MenuObject,
        point: egui::Pos2,
        confirmed: bool,
    },
}

#[derive(Default)]
struct WindowDrawResult {
    focus: bool,
    close: bool,
    switch_side: bool,
    selected: Option<SystemKey>,
    opened: Option<(SystemKey, (i16, i16))>,
    /// A double click on a Defenses, Fleet or Missions icon.
    opened_icon: Option<(SystemKey, Quadrant, (i16, i16))>,
    /// A left or right press on a shown icon.
    icon_pressed: Option<SelectedIcon>,
    /// The window's selected icon hid or changed its art.
    selection_lapsed: bool,
    /// A right-button release in the window, as a canvas point.
    object_menu: Option<(i16, i16)>,
    /// A left press on a shown icon, as a canvas point.
    drag_from: Option<(i16, i16)>,
    /// A left release ended the window's drag: where, and whether Ctrl was
    /// held, when it went far enough to drop.
    drag_end: Option<Option<(egui::Pos2, bool)>>,
    /// The held drag's image: the bitmap and the icon's rect moved with it.
    drag_image: Option<(u32, egui::Rect)>,
    /// The headquarters overlays painted, each the system and its planet's
    /// rect.
    headquarters: Vec<(SystemKey, egui::Rect)>,
}

/// Paint and operate all open sector windows using the recovered strategic
/// canvas. Window mutations are applied after the pass so area ordering stays
/// deterministic.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and rendering inputs at this existing UI boundary."
)]
pub fn draw_sector_windows(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &rebellion_core::movement::MovementState,
    travellers: crate::quadrant_icons::Travellers<'_>,
    fog: &FogState,
    state: &mut SectorWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    uprisings: &rebellion_core::uprising::UprisingState,
    missions: &rebellion_core::missions::MissionState,
) -> Vec<SectorWindowAction> {
    state.prepare_faction(faction);
    let windows = state.windows.clone();
    let mut actions = Vec::new();
    let mut focused = None;
    let mut closed = None;
    let mut switched = None;
    let mut chosen = Vec::new();
    let mut drag_image = None;
    let mut headquarters_marks = Vec::new();

    let focused_sector = windows.last().map(|window| window.sector);
    for window in windows {
        let result = draw_sector_window(
            ctx,
            world,
            movement,
            travellers,
            fog,
            window,
            focused_sector == Some(window.sector),
            faction,
            layout,
            cache,
            uprisings,
            missions,
            state.backdrop,
            &state.gid_markers,
            &state.drawn_gid_markers,
            state.gid_caption.as_ref(),
        );
        if result.focus {
            focused = Some(window.sector);
        }
        if result.close {
            closed = Some(window.sector);
        }
        if result.switch_side {
            switched = Some(window.sector);
        }
        drag_image = drag_image.or(result.drag_image);
        headquarters_marks.extend(
            result
                .headquarters
                .iter()
                .map(|&(system, rect)| (area_id(window.sector), system, rect)),
        );
        if let Some(system) = result.selected {
            actions.push(SectorWindowAction::SelectSystem(system));
        }
        if let Some((system, logical_position)) = result.opened {
            actions.push(quadrant_window_action(
                system,
                Quadrant::System,
                logical_position,
            ));
        }
        if let Some((system, quadrant, logical_position)) = result.opened_icon {
            actions.push(quadrant_window_action(system, quadrant, logical_position));
        }
        let selection = result
            .icon_pressed
            .or(window.selection.filter(|_| !result.selection_lapsed));
        // A press on an icon sets the selection and the drag; a release
        // ends the drag.
        if result.icon_pressed.is_some() || result.drag_end.is_some() || result.selection_lapsed {
            chosen.push((window.sector, selection, result.drag_from));
        }
        let selection_object = selection.and_then(|icon| selected_object(world, icon));
        if let (Some(Some((point, confirmed))), Some(selection)) =
            (result.drag_end, selection_object)
        {
            actions.push(SectorWindowAction::DropSelection {
                selection,
                point,
                confirmed,
            });
        }
        if let Some(point) = result.object_menu {
            actions.push(SectorWindowAction::OpenObjectMenu {
                selection: selection_object,
                point,
            });
        }
    }

    state.drag_image = drag_image;
    state.headquarters_marks = headquarters_marks;
    for (sector, selection, drag_from) in chosen {
        if let Some(window) = state
            .windows
            .iter_mut()
            .find(|window| window.sector == sector)
        {
            window.selection = selection;
            window.drag_from = drag_from;
        }
    }

    if let Some(sector) = closed {
        state.close(sector);
    } else if let Some(sector) = switched {
        state.switch_side(sector);
    } else if let Some(sector) = focused {
        state.focus(sector);
    }
    actions
}

/// What the galaxy view drew, which a child window's copy of it shows
/// shaded: the starfield (STRATEGY 902 or 903), the markers it drew and
/// their screen centres, and its caption.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShadedView<'a> {
    pub backdrop: u32,
    pub drawn_gid_markers: &'a [(u32, [f32; 2])],
    pub gid_caption: Option<&'a crate::GidCaption>,
}

/// Paint the galaxy view's picture under `window_rect`, shaded: the copy a
/// child window takes of its parent (`FUN_005fcc30`) mapped through the
/// shading table `FUN_005fddd0(LAB_004ac550)` builds (`FUN_005fe050`). The
/// sector window (`FUN_004593e0`) and the GID legend (`FUN_00452240`) both
/// paint it under their art. port: the copy holds the starfield, the GID
/// markers and the caption, not other windows under it.
pub(crate) fn paint_shaded_view(
    ctx: &egui::Context,
    painter: &egui::Painter,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    window_rect: egui::Rect,
    view: &ShadedView<'_>,
) {
    let ShadedView {
        backdrop,
        drawn_gid_markers,
        gid_caption,
    } = *view;
    painter.rect_filled(window_rect, 0.0, egui::Color32::from_rgb(42, 42, 42));
    if let Some(texture) = cache.get_shaded(ctx, DllSource::Strategy, backdrop) {
        let size = texture.size_vec2();
        let starfield = crate::galaxy_backdrop_destination(layout, faction, size.x, size.y);
        let origin = egui::pos2(starfield.x, starfield.y);
        let extent = egui::vec2(starfield.width, starfield.height);
        let uv = egui::Rect::from_min_max(
            ((window_rect.min - origin) / extent).to_pos2(),
            ((window_rect.max - origin) / extent).to_pos2(),
        );
        painter.image(texture.id(), window_rect, uv, egui::Color32::WHITE);
    }
    // The view's markers under the window, hung from their cell's top
    // left as the view drew them (`FUN_0042cde0`), shaded with the
    // rest of the copy.
    for &(marker, [x, y]) in drawn_gid_markers {
        if let Some(texture) = cache.get_shaded(ctx, DllSource::Strategy, marker) {
            let size = texture.size_vec2() * layout.scale;
            let corner =
                egui::vec2(crate::GALAXY_MAP_OFFSET.0, crate::GALAXY_MAP_OFFSET.1) * layout.scale;
            let rect = egui::Rect::from_min_size(egui::pos2(x, y) - corner, size);
            if rect.intersects(window_rect) {
                painter.image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
    }
    // The view's caption (FUN_00425d00 draws it into the view's
    // picture), shaded with the rest of the copy: A0
    // sector-windows/02-second shows "Popular Support" (255, 0, 0)
    // as (91, 67, 51) under both windows, the starfield palette's
    // entry FUN_005fddd0's table maps red to.
    if let Some(caption) = gid_caption {
        let [red, green, blue, _] = caption.color.to_array();
        if let Some([red, green, blue]) =
            cache.shaded_palette_color(DllSource::Strategy, backdrop, [red, green, blue])
        {
            caption.paint(painter, egui::Color32::from_rgb(red, green, blue));
        }
    }
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
fn draw_sector_window(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &rebellion_core::movement::MovementState,
    travellers: crate::quadrant_icons::Travellers<'_>,
    fog: &FogState,
    window: OpenSectorWindow,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    uprisings: &rebellion_core::uprising::UprisingState,
    missions: &rebellion_core::missions::MissionState,
    backdrop: u32,
    gid_markers: &std::collections::HashMap<SystemKey, u32>,
    drawn_gid_markers: &[(u32, [f32; 2])],
    gid_caption: Option<&crate::GidCaption>,
) -> WindowDrawResult {
    let mut result = WindowDrawResult::default();
    let Some(sector) = world.sectors.get(window.sector) else {
        result.close = true;
        return result;
    };
    let position = window_logical_position(faction, window.column);
    let screen_position = egui::pos2(
        layout.canvas.x + position.0 * layout.scale,
        layout.canvas.y + position.1 * layout.scale,
    );
    let size = egui::vec2(
        SECTOR_WINDOW_WIDTH * layout.scale,
        SECTOR_WINDOW_HEIGHT * layout.scale,
    );

    let area_id = area_id(window.sector);
    if focused {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, area_id));
    }
    let area = egui::Area::new(area_id)
        .fixed_pos(screen_position)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let (pointer, primary_down, primary_pressed, any_pressed, released, ctrl) =
                ctx.input(|input| {
                    (
                        input.pointer.interact_pos(),
                        input.pointer.button_down(egui::PointerButton::Primary),
                        input.pointer.button_pressed(egui::PointerButton::Primary),
                        input.pointer.button_pressed(egui::PointerButton::Primary)
                            || input.pointer.button_pressed(egui::PointerButton::Secondary),
                        (
                            input.pointer.button_released(egui::PointerButton::Primary),
                            input
                                .pointer
                                .button_released(egui::PointerButton::Secondary),
                        ),
                        input.modifiers.ctrl,
                    )
                });
            let (primary_released, secondary_released) = released;
            let (window_rect, window_response) = ui.allocate_exact_size(size, egui::Sense::click());
            // The original is a child window (FUN_00459e30 places its items
            // in it), so a planet near its bottom edge is cut off there.
            ui.set_clip_rect(window_rect);
            // WM_PAINT (FUN_004593e0) copies the galaxy view's picture under
            // the window (its +0x45c at the window's +0x28, +0x2c) and maps
            // it through the shading table FUN_00459e30 built
            // (FUN_005fddd0(LAB_004ac550), FUN_005fe050).
            paint_shaded_view(
                ctx,
                ui.painter(),
                cache,
                layout,
                faction,
                window_rect,
                &ShadedView {
                    backdrop,
                    drawn_gid_markers,
                    gid_caption,
                },
            );
            // FUN_00459e30 frames the window with FUN_00607740's tiles, as
            // the GID's menus are (A0 sector-windows/02-second: solid top
            // and bottom, sides on every second row).
            crate::cockpit::paint_gid_frame_border(ui, cache, window_rect, layout.scale);

            // FUN_00459e30: the sector's name in font 4, 0x200f0f0 (yellow),
            // centred (0x25: DT_CENTER | DT_VCENTER | DT_SINGLELINE) in the
            // title from (2, 2), 20 high (the text object's +4, 0x14).
            // The text object is the window's width from (2, 2): A0's titles
            // centre on x 119.5 (Churba and Sesswenna at 13 ppem).
            crate::theme::paint_gdi_text(
                ui.painter(),
                logical_rect(window_rect, layout.scale, 2.0, 2.0, 235.0, 20.0),
                layout.scale,
                4,
                &sector.name,
                SECTOR_TITLE_COLOR,
                (
                    crate::theme::GdiAlign::Center,
                    crate::theme::GdiVAlign::VCenter,
                ),
            );

            let switch_rect = logical_rect(window_rect, layout.scale, 204.0, 2.0, 14.0, 14.0);
            let close_rect = logical_rect(window_rect, layout.scale, 218.0, 2.0, 14.0, 14.0);
            let switch_response = ui.interact(
                switch_rect,
                ui.id().with((window.sector, "switch")),
                egui::Sense::click(),
            );
            let close_response = ui.interact(
                close_rect,
                ui.id().with((window.sector, "close")),
                egui::Sense::click(),
            );
            let switch_pressed =
                primary_down && pointer.is_some_and(|point| rect_contains(switch_rect, point));
            let close_pressed =
                primary_down && pointer.is_some_and(|point| rect_contains(close_rect, point));
            let switch_clicked = exact_clicked(&switch_response, switch_rect);
            let close_clicked = exact_clicked(&close_response, close_rect);
            paint_resource(
                ui.painter(),
                ctx,
                cache,
                if switch_pressed {
                    SWITCH_SIDE_PRESSED
                } else {
                    SWITCH_SIDE_NORMAL
                },
                switch_rect,
            );
            paint_resource(
                ui.painter(),
                ctx,
                cache,
                if close_pressed {
                    CLOSE_PRESSED
                } else {
                    CLOSE_NORMAL
                },
                close_rect,
            );

            let player = cockpit_faction(faction);
            let layer = egui::LayerId::new(egui::Order::Middle, area_id);
            let double_click = ctx
                .input(|input| {
                    input
                        .pointer
                        .button_double_clicked(egui::PointerButton::Primary)
                })
                .then_some(pointer)
                .flatten()
                .filter(|point| ctx.layer_id_at(*point) == Some(layer));
            // The window's area is exactly its rect, so its layer under the
            // point also says no other window covers it.
            let on_window = |point: &egui::Pos2| ctx.layer_id_at(*point) == Some(layer);
            // FUN_004593e0 cases 0x201 and 0x204 hit-test the overlays
            // (FUN_0045cc10); a press that finds none keeps the selection.
            let press = any_pressed.then_some(pointer).flatten().filter(on_window);
            // WM_PAINT draws the planet list (`+0x164`), then the overlay
            // list (`+0x174`), so every icon lies above every planet.
            let mut overlays: Vec<(SystemKey, Quadrant, egui::Rect, (u32, u32))> = Vec::new();
            let mut headquarters: Vec<(SystemKey, egui::Rect)> = Vec::new();
            // FUN_0045cc10 walks the overlay list, sorted by id (mode 1,
            // Class_0066e148_vt4), so the headquarters overlay (0x10000)
            // comes before every quadrant icon (sys_id | 0x40000 and up)
            // and takes a point wherever its bitmap is opaque.
            let headquarters_at = |point: egui::Pos2, cache: &mut BmpCache| {
                sector.systems.iter().copied().find(|system_key| {
                    world.systems.get(*system_key).is_some_and(|system| {
                        let (planet_x, planet_y) =
                            sector_planet_position(sector.x, sector.y, system.x, system.y);
                        shows_alliance_headquarters(world, fog, player, *system_key)
                            && headquarters_hit(
                                cache,
                                planet_rect(window_rect, layout.scale, planet_x, planet_y),
                                layout.scale,
                                point,
                            )
                    })
                })
            };
            let headquarters_pressed = press.and_then(|point| headquarters_at(point, cache));
            let headquarters_double_clicked =
                double_click.is_some_and(|point| headquarters_at(point, cache).is_some());
            if let Some(system) = headquarters_pressed {
                result.icon_pressed = Some(SelectedIcon {
                    system,
                    quadrant: None,
                    art: HEADQUARTERS_MARK,
                    contents: system_contents(world, system),
                });
                if primary_pressed {
                    result.drag_from = press.map(|point| screen_to_logical(layout, point));
                }
            }
            for system_key in &sector.systems {
                // FUN_0045bbb0 reads the player's copy (FUN_004f3220): its
                // name colour, destroyed art and tracks follow the copy.
                let Some(shown) =
                    crate::viewed_contents::apparent_system(world, player, *system_key)
                else {
                    continue;
                };
                let system: &rebellion_core::world::System = &shown;
                let (planet_x, planet_y) =
                    sector_planet_position(sector.x, sector.y, system.x, system.y);
                let planet_rect = planet_rect(window_rect, layout.scale, planet_x, planet_y);
                let planet_response = ui.interact(
                    planet_rect,
                    ui.id().with((window.sector, *system_key)),
                    egui::Sense::click(),
                );
                let planet_hovered = pointer.is_some_and(|point| rect_contains(planet_rect, point));
                let planet_clicked = exact_clicked(&planet_response, planet_rect);
                // FUN_0045d140 draws an overlay only while its rule has
                // something to show; a double click finds it first
                // (`FUN_0045cc10`).
                let icons: Vec<(Quadrant, egui::Rect, (u32, u32))> = Quadrant::ALL
                    .into_iter()
                    .filter_map(|quadrant| {
                        let art = quadrant_icon(
                            world,
                            fog,
                            missions,
                            movement,
                            travellers,
                            player,
                            *system_key,
                            quadrant,
                        )?;
                        Some((
                            quadrant,
                            quadrant_rect(planet_rect, layout.scale, quadrant),
                            art,
                        ))
                    })
                    .collect();
                // port: FUN_0045d140 deselects an icon (FUN_0045afc0) on every
                // change notice FUN_0045b770 takes for its system or an object
                // at it, changed or not. The port has no change notices, so the
                // selection lapses when its icon hides or its art changes, or
                // an object arrives at or leaves its system
                // (ghidra/notes/sector-quadrants.md; A0
                // 2026-11-13-defenses-missions-drop/05).
                if let Some(selected) = window.selection.filter(|icon| icon.system == *system_key) {
                    let moved = selected.contents != system_contents(world, *system_key);
                    result.selection_lapsed = moved
                        || match selected.quadrant {
                            Some(chosen) => !icons.iter().any(|(quadrant, _, (normal, _))| {
                                *quadrant == chosen && *normal == selected.art
                            }),
                            None => !shows_alliance_headquarters(world, fog, player, *system_key),
                        };
                }
                // FUN_0045aac0 opens nothing for the headquarters' kind 1,
                // but FUN_0045cc10 found it first.
                let icon_double_clicked = double_click
                    .filter(|_| !headquarters_double_clicked)
                    .and_then(|point| {
                        icons
                            .iter()
                            .find(|(_, rect, _)| rect_contains(*rect, point))
                            .map(|(quadrant, rect, _)| (*quadrant, *rect))
                    });
                if let Some((quadrant, _, (normal, _))) = press
                    .filter(|_| headquarters_pressed.is_none())
                    .and_then(|point| {
                        icons
                            .iter()
                            .find(|(_, rect, _)| rect_contains(*rect, point))
                    })
                {
                    result.icon_pressed = Some(SelectedIcon {
                        system: *system_key,
                        quadrant: Some(*quadrant),
                        art: *normal,
                        contents: system_contents(world, *system_key),
                    });
                    if primary_pressed {
                        result.drag_from = press.map(|point| screen_to_logical(layout, point));
                    }
                }
                paint_resource(ui.painter(), ctx, cache, planet_art(system), planet_rect);
                // FUN_0045bbb0 blits a side's uprising mark (0x389 side 1,
                // 0x38a side 2) keyed into the planet's bitmap at (0, 0).
                if let Some(mark) = uprising_mark(world, player, uprisings, *system_key) {
                    paint_native(
                        ui.painter(),
                        ctx,
                        cache,
                        DllSource::Strategy,
                        mark,
                        planet_rect,
                        layout.scale,
                        0.0,
                        0.0,
                    );
                }
                // FUN_0045bbb0 then blits the system's galaxy-view marker
                // (FUN_0042d860, the map item's bitmap) at the bottom left of
                // the planet's bitmap, made 4 rows taller than the picture.
                if let Some(&marker) = gid_markers.get(system_key).filter(|_| !system.is_destroyed)
                {
                    let height = cache
                        .original_resource_size(DllSource::Strategy, marker)
                        .map_or(0.0, |[_, height]| height as f32);
                    paint_native(
                        ui.painter(),
                        ctx,
                        cache,
                        DllSource::Strategy,
                        marker,
                        planet_rect,
                        layout.scale,
                        0.0,
                        PLANET_SIZE + 4.0 - height,
                    );
                }
                if shows_alliance_headquarters(world, fog, player, *system_key) {
                    headquarters.push((*system_key, planet_rect));
                }
                let track_values = StatusTrackValues::shown(world, fog, system, *system_key, player);
                for (row, (x, y, width, height), text) in track_tooltips(&track_values) {
                    if track_shown(system, row) {
                        crate::win_tooltip::window_tool_text(
                            ui,
                            logical_rect(
                                window_rect,
                                layout.scale,
                                planet_x + x,
                                planet_y + y,
                                width,
                                height,
                            ),
                            (*system_key, row.to_bits()),
                            &text,
                        );
                    }
                }
                for ((x, y, width, height), color) in
                    status_tracks(planet_x, planet_y, &track_values, player)
                        .into_iter()
                        .filter(|&((_, row, _, _), _)| track_shown(system, row - planet_y))
                {
                    ui.painter().rect_filled(
                        logical_rect(window_rect, layout.scale, x, y, width, height),
                        0.0,
                        color,
                    );
                }
                // FUN_004593e0: centred in a 100-wide box at the bitmap's
                // x + width / 2 - 50 (37 / 2 = 18), from the label's top.
                crate::theme::paint_gdi_text(
                    ui.painter(),
                    logical_rect(
                        window_rect,
                        layout.scale,
                        planet_x + 18.0 - 50.0,
                        planet_y + NAME_OFFSET,
                        100.0,
                        16.0,
                    ),
                    layout.scale,
                    10,
                    &system.name,
                    if system.is_destroyed {
                        DESTROYED_NAME_COLOR
                    } else {
                        system_name_color(system)
                    },
                    (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
                );

                overlays.extend(
                    icons
                        .iter()
                        .map(|(quadrant, rect, art)| (*system_key, *quadrant, *rect, *art)),
                );
                if let Some((quadrant, icon_rect)) = icon_double_clicked {
                    let open_point = double_click.unwrap_or_else(|| icon_rect.center());
                    let open_point = screen_to_logical(layout, open_point);
                    if quadrant == Quadrant::System {
                        result.opened = Some((*system_key, open_point));
                    } else {
                        result.opened_icon = Some((*system_key, quadrant, open_point));
                    }
                    result.focus = true;
                }

                if planet_hovered || planet_clicked {
                    paint_selection_brackets(ui.painter(), planet_rect, layout.scale);
                }
                if planet_clicked {
                    result.selected = Some(*system_key);
                    result.focus = true;
                }
                // FUN_004593e0 case 0x203 hit-tests only the overlay list
                // (FUN_0045cc10, +0x174): a double click on the planet
                // itself opens nothing (A0 2026-11-09: Adega, Flax and
                // Akrit'tar; system-windows/04, 05 Yavin). The System
                // window opens from its icon.
            }

            // WM_PAINT draws a shown overlay's second bitmap (`+0x24`) while
            // it is selected (`+0x3c` bit 0, set by FUN_0045b1b0), else its
            // first (`+0x20`).
            let selected = result
                .icon_pressed
                .or(window.selection.filter(|_| !result.selection_lapsed));
            // FUN_0045bbb0 adds the headquarters overlay (id 0x10000, bitmap
            // 0x388) on the planet's rect.
            result.headquarters.clone_from(&headquarters);
            for &(_, rect) in &headquarters {
                paint_native(
                    ui.painter(),
                    ctx,
                    cache,
                    DllSource::Strategy,
                    HEADQUARTERS_MARK,
                    rect,
                    layout.scale,
                    0.0,
                    0.0,
                );
            }
            for &(system, quadrant, rect, (normal, pressed)) in &overlays {
                let chosen = selected
                    .is_some_and(|icon| icon.system == system && icon.quadrant == Some(quadrant));
                paint_native(
                    ui.painter(),
                    ctx,
                    cache,
                    DllSource::Strategy,
                    if chosen { pressed } else { normal },
                    rect,
                    layout.scale,
                    0.0,
                    0.0,
                );
            }

            // FUN_004593e0 case 0x200: the first move with the button held
            // (the window holds the capture) starts the drag image
            // (FUN_0060dbb0, FUN_0060dcb0, FUN_0060dce0) from the window-
            // sized bitmap FUN_0045b1b0 blitted the chosen icon's first
            // bitmap (+0x20, not the pressed +0x24) into at the icon's
            // rect; later moves slide it with the pointer (FUN_0060df00).
            // The icon itself keeps its pressed art. The image paints over
            // every window (FUN_0060dce0 clears the frame's
            // WS_CLIPCHILDREN), cut to the window's bitmap.
            if let (true, Some(from), Some(point), Some(icon)) =
                (primary_down, window.drag_from, pointer, selected)
            {
                let press = egui::pos2(
                    layout.canvas.x + f32::from(from.0) * layout.scale,
                    layout.canvas.y + f32::from(from.1) * layout.scale,
                );
                let offset = point - press;
                let source = match icon.quadrant {
                    Some(chosen) => overlays
                        .iter()
                        .find(|(system, quadrant, _, _)| {
                            *system == icon.system && *quadrant == chosen
                        })
                        .map(|&(_, _, rect, (normal, _))| (normal, rect)),
                    None => headquarters
                        .iter()
                        .find(|(system, _)| *system == icon.system)
                        .map(|&(_, rect)| (HEADQUARTERS_MARK, rect)),
                };
                if let (true, Some((art, rect))) = (offset != egui::Vec2::ZERO, source) {
                    let painter = ctx
                        .layer_painter(crate::fleet_window::drag_layer())
                        .with_clip_rect(window_rect.translate(offset));
                    paint_native(
                        &painter,
                        ctx,
                        cache,
                        DllSource::Strategy,
                        art,
                        rect.translate(offset),
                        layout.scale,
                        0.0,
                        0.0,
                    );
                    result.drag_image = Some((art, rect.translate(offset)));
                }
            }
            // FUN_004593e0 case 0x202: the window holds the capture, so the
            // release ends the drag wherever it lands; it drops only past
            // four pixels from the press in x or y. A press whose release
            // came in the same frame (both messages of a quick click
            // between two slow frames) is ended by that release: no drag
            // outlives the button (case 0x202's ReleaseCapture).
            let held_from = window
                .drag_from
                .or(result.drag_from.filter(|_| !primary_down));
            if let (true, Some(from)) = (primary_released, held_from) {
                if !primary_down {
                    result.drag_from = None;
                }
                result.drag_end = Some(pointer.and_then(|point| {
                    let to = screen_to_logical(layout, point);
                    ((to.0 - from.0).abs() > 4 || (to.1 - from.1).abs() > 4)
                        .then_some((point, ctrl))
                }));
            }
            // WM_RBUTTONUP: slot 7, FUN_004ac5c0, opens the menu for the
            // window's selection, whatever lies under the cursor.
            if let Some(point) = secondary_released
                .then_some(pointer)
                .flatten()
                .filter(on_window)
            {
                result.object_menu = Some(screen_to_logical(layout, point));
                result.focus = true;
            }
            if window_response.clicked() || switch_clicked || close_clicked {
                result.focus = true;
            }
            result.switch_side = switch_clicked;
            result.close = close_clicked;
        });

    if area.response.clicked() {
        result.focus = true;
    }
    result
}

fn window_logical_position(faction: CockpitFaction, column: WindowColumn) -> (f32, f32) {
    match (faction, column) {
        (CockpitFaction::Alliance, WindowColumn::Primary) => (60.0, 35.0),
        (CockpitFaction::Alliance, WindowColumn::Secondary) => (300.0, 35.0),
        (CockpitFaction::Empire, WindowColumn::Primary) => (120.0, 40.0),
        (CockpitFaction::Empire, WindowColumn::Secondary) => (365.0, 40.0),
    }
}

fn area_id(sector: SectorKey) -> egui::Id {
    egui::Id::new(("original-sector-window", sector))
}

/// A planet picture's side: every SYSTEMSD picture and the asteroid field
/// are 37 by 37.
const PLANET_SIZE: f32 = 37.0;

/// A planet item's picture and hit rectangle: 37 by 37 at its position.
fn planet_rect(window_rect: egui::Rect, scale: f32, x: f32, y: f32) -> egui::Rect {
    logical_rect(window_rect, scale, x, y, PLANET_SIZE, PLANET_SIZE)
}

/// A quadrant overlay around a planet whose center is (x + 18, y + 18),
/// 28 by 19, the 27 by 18 icon 10771 plus one (`FUN_00459e30:369-447`):
/// the left ones end at `cx`, the right ones start at `cx + 1`; the top ones
/// end at `cy`, the bottom ones start at `cy + 1`.
/// The window a double click on `system`'s `quadrant` icon opens at
/// `logical_position`. `FUN_0045aac0`: kind 4 opens the System window,
/// kind 8 the Defenses window, kind 0x10 the Fleet window and kind 0x40 the
/// Missions window.
fn quadrant_window_action(
    system: SystemKey,
    quadrant: Quadrant,
    logical_position: (i16, i16),
) -> SectorWindowAction {
    match quadrant {
        Quadrant::System => SectorWindowAction::OpenSystemWindow {
            system,
            logical_position,
        },
        Quadrant::Defenses => SectorWindowAction::OpenDefensesWindow {
            system,
            logical_position,
        },
        Quadrant::Fleets => SectorWindowAction::OpenFleetWindow {
            system,
            logical_position,
        },
        Quadrant::Missions => SectorWindowAction::OpenMissionsWindow {
            system,
            logical_position,
        },
    }
}

/// A quadrant item's rectangle from its planet's (FUN_0045d140). A0
/// 2026-10-28-nav/sector-a (Vjun at (230, 133)): the System, Defenses and
/// Fleets items at (220, 132), (220, 152) and (249, 132), every non-key
/// pixel exact.
fn quadrant_rect(planet: egui::Rect, scale: f32, quadrant: Quadrant) -> egui::Rect {
    let (x, y) = match quadrant {
        Quadrant::System => (-10.0, -1.0),
        Quadrant::Defenses => (-10.0, 19.0),
        Quadrant::Fleets => (19.0, -1.0),
        Quadrant::Missions => (19.0, 19.0),
    };
    egui::Rect::from_min_size(
        planet.min + egui::vec2(x, y) * scale,
        egui::vec2(28.0, 19.0) * scale,
    )
}

fn window_screen_rect(
    faction: CockpitFaction,
    column: WindowColumn,
    layout: CockpitLayout,
) -> egui::Rect {
    let position = window_logical_position(faction, column);
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + position.0 * layout.scale,
            layout.canvas.y + position.1 * layout.scale,
        ),
        egui::vec2(
            SECTOR_WINDOW_WIDTH * layout.scale,
            SECTOR_WINDOW_HEIGHT * layout.scale,
        ),
    )
}

fn logical_rect(
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        logical_point(parent, scale, x, y),
        egui::vec2(width * scale, height * scale),
    )
}

fn logical_point(parent: egui::Rect, scale: f32, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(parent.min.x + x * scale, parent.min.y + y * scale)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
fn screen_to_logical(layout: CockpitLayout, point: egui::Pos2) -> (i16, i16) {
    let scale = layout.scale.max(f32::EPSILON);
    (
        ((point.x - layout.canvas.x) / scale).round() as i16,
        ((point.y - layout.canvas.y) / scale).round() as i16,
    )
}

fn exact_clicked(response: &egui::Response, rect: egui::Rect) -> bool {
    response.clicked()
        && response
            .interact_pointer_pos()
            .is_some_and(|point| rect_contains(rect, point))
}

fn rect_contains(rect: egui::Rect, point: egui::Pos2) -> bool {
    point.x >= rect.min.x && point.x < rect.max.x && point.y >= rect.min.y && point.y < rect.max.y
}

/// A planet's position in its sector window (`FUN_00459e30` at
/// `0x45a3d4`): the system's offset from its sector (`FUN_00509620`,
/// `FUN_00526560`) over 1024, times 13 by the window's 235-pixel width
/// across and 10 by its 360-pixel height down, truncated (`__ftol`).
#[expect(
    clippy::cast_possible_truncation,
    reason = "__ftol truncates toward zero, as the cast does."
)]
fn sector_planet_position(
    sector_x: u16,
    sector_y: u16,
    system_x: u16,
    system_y: u16,
) -> (f32, f32) {
    let scaled = |system: u16, sector: u16, factor: f64, extent: f32| {
        let offset = f64::from(i32::from(system) - i32::from(sector));
        (offset / 1024.0 * factor * f64::from(extent)) as i32 as f32
    };
    (
        scaled(system_x, sector_x, 13.0, SECTOR_WINDOW_WIDTH),
        scaled(system_y, sector_y, 10.0, SECTOR_WINDOW_HEIGHT),
    )
}

/// STRATEGY 10240 (`0x2800`), the asteroid field `FUN_0045bbb0` shows in
/// place of a destroyed system's planet (`+0x50` bit 3, item state `0x40`).
const DESTROYED_PLANET: u32 = 10240;
/// `FUN_0045bbb0`'s name colour for a destroyed system, `0x2ffff00`, the
/// one it gives an unheld system: A0 draws both white (menus3/05, Averam
/// after the Death Star; menus3/00, the neutral systems).
const DESTROYED_NAME_COLOR: egui::Color32 = egui::Color32::WHITE;

/// A system's art in the sector window: its planet (`FUN_0045c970` of its
/// SYSTEMSD picture), or the asteroid field once it is destroyed
/// (`FUN_0045bbb0`).
pub(crate) fn planet_art(system: &rebellion_core::world::System) -> u32 {
    if system.is_destroyed {
        DESTROYED_PLANET
    } else {
        planet_resource_id(system.dat_id)
    }
}

pub(crate) fn planet_resource_id(dat_id: DatId) -> u32 {
    let picture = planet_picture_id(dat_id);
    match picture {
        1..=23 => 10211 + u32::from(picture),
        24 => 10239,
        25 => 10237,
        26 => 10238,
        _ => 10212,
    }
}

fn cockpit_faction(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

/// `FUN_0045bbb0`'s name colour by the system's side bits: `0x20000ff`
/// (red) for side 1, `0x200ff00` (green) for side 2. Any other side leaves
/// the item's constructor colour, white (`0xffffff`, `FUN_0060ba00`): the A0
/// Sesswenna capture shows the neutral systems' names white and the Alliance
/// and Empire systems' red and green. port: an unexplored system shows no
/// side, as its overlays do (`quadrant_icons`), so its name stays white
/// (the A0 Moddell capture).
fn system_name_color(system: &rebellion_core::world::System) -> egui::Color32 {
    if system.exploration_status == rebellion_core::dat::ExplorationStatus::Unexplored {
        return egui::Color32::WHITE;
    }
    match crate::fleet_window::control_side(system.control) {
        1 => egui::Color32::from_rgb(255, 0, 0),
        2 => egui::Color32::from_rgb(0, 255, 0),
        _ => egui::Color32::WHITE,
    }
}

/// The headquarters mark, STRATEGY 904 (`0x388`), 37 by 37.
const HEADQUARTERS_MARK: u32 = 904;
/// The uprising marks (`0x389` side 1, `0x38a` side 2), 37 by 37.
const UPRISING_MARKS: [u32; 2] = [905, 906];

/// `FUN_0045bbb0`: a system in uprising (`+0x88` bit 2) held by side 1 or 2
/// takes that side's mark, all read from the player's copy (`FUN_004f3220`):
/// a frozen copy shows the uprising it was last synced with.
fn uprising_mark(
    world: &GameWorld,
    player: Faction,
    uprisings: &rebellion_core::uprising::UprisingState,
    system: SystemKey,
) -> Option<u32> {
    let value = crate::viewed_contents::apparent_system(world, player, system)?;
    if value.is_destroyed
        || !rebellion_core::object_views::side_uprising(world, player, system, uprisings)
    {
        return None;
    }
    match crate::fleet_window::control_side(value.control) {
        1 => Some(UPRISING_MARKS[0]),
        2 => Some(UPRISING_MARKS[1]),
        _ => None,
    }
}

/// Whether `point` takes the headquarters overlay over `rect` (the planet's
/// rect, `+0x40`): `FUN_005fca00` tests the bitmap's pixel at the point's
/// offset against its transparent key, and the hit mask, as it does, wants
/// the offset above 0 in x and y.
fn headquarters_hit(cache: &mut BmpCache, rect: egui::Rect, scale: f32, point: egui::Pos2) -> bool {
    if !rect_contains(rect, point) {
        return false;
    }
    let x = ((point.x - rect.min.x) / scale).floor() as usize;
    let y = ((point.y - rect.min.y) / scale).floor() as usize;
    cache.is_resource_hit(DllSource::Strategy, HEADQUARTERS_MARK, x, y)
}

/// What lies at `system`: its fleets, regiments, squadrons, special
/// forces, facilities and the characters there, as a key that changes when
/// one of them arrives or leaves. port: it stands in for the change notices
/// `FUN_0045b770` takes for the system or an object at it, each of which
/// deselects its icon (`FUN_0045d140` -> `FUN_0045afc0`).
fn system_contents(world: &GameWorld, system: SystemKey) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    if let Some(value) = world.systems.get(system) {
        value.fleets.hash(&mut hasher);
        value.ground_units.hash(&mut hasher);
        for entry in &value.fighters {
            (entry.class, entry.count).hash(&mut hasher);
        }
        value.special_forces.hash(&mut hasher);
        value.defense_facilities.hash(&mut hasher);
        value.manufacturing_facilities.hash(&mut hasher);
        value.production_facilities.hash(&mut hasher);
    }
    for (key, character) in &world.characters {
        if character.current_system == Some(system) {
            key.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// The menu object of a window's selection: its quadrant icon's system, or
/// the headquarters object in the system ([`headquarters_object`]).
fn selected_object(world: &GameWorld, icon: SelectedIcon) -> Option<MenuObject> {
    match icon.quadrant {
        Some(quadrant) => Some(MenuObject::SystemIcon {
            system: icon.system,
            quadrant,
        }),
        None => headquarters_object(world, icon.system).map(MenuObject::Headquarters),
    }
}

/// The headquarters object (family `0x20`) among `system`'s facilities, as
/// `FUN_0052bed0` gives the overlay its id.
#[must_use]
pub fn headquarters_object(
    world: &GameWorld,
    system: SystemKey,
) -> Option<rebellion_core::ids::ManufacturingFacilityKey> {
    world
        .systems
        .get(system)?
        .manufacturing_facilities
        .iter()
        .copied()
        .find(|key| {
            world
                .manufacturing_facilities
                .get(*key)
                .is_some_and(|facility| facility.class_dat_id.family() == 0x20)
        })
}

/// `FUN_0045bbb0`'s item state bit 8: the viewer's record of the system
/// (`FUN_004f3220` on the side's table, `+0x9c`) holds a headquarters
/// object (families `0x20..0x21`, `FUN_00526cf0(record, 3)`), which serves
/// only the Alliance (`ghidra/notes/facility-ownership.md`). So the Empire
/// sees the mark while its copy of the system is live, and keeps seeing it
/// in a frozen copy made while the headquarters stood there (A0 2026-11-16-
/// edited-saves/x-39, Tokmia under the Empire's blockade; x-29, Duros on
/// day 527). port: a live view's headquarters is the Alliance-held system
/// marked `is_headquarters`, which the Alliance always sees (it holds the
/// system); a frozen copy's is the headquarters object it copied.
///
/// A system the port's fog alone shows (no copy) holds no headquarters for
/// the Empire: the setup copies (`FUN_005136d0` stage `0x18` ->
/// `FUN_0051b990`) hold no facility, and the HQ object's copy turns live
/// only with the system's seen bit (`FUN_004f7c70` -> `FUN_0050ac80`), so
/// only a live copy, a pushed or frozen one, or the player's own system
/// shows it (`ghidra/notes/headquarters-overlay.md`, "Can the Empire see
/// 904").
fn shows_alliance_headquarters(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> bool {
    use crate::viewed_contents::{sees_everything, system_sight, SystemSight};
    let Some(value) = world.systems.get(system) else {
        return false;
    };
    if value.is_destroyed {
        return false;
    }
    match system_sight(world, fog, player, system) {
        SystemSight::Live => {
            let copy_live = sees_everything(world)
                || value.control.is_controlled_by(player)
                || world
                    .object_views
                    .system(player, value.dat_id)
                    .is_some_and(|view| view.live);
            copy_live && value.is_headquarters && value.control.is_controlled_by(Faction::Alliance)
        }
        SystemSight::Frozen(view) => view
            .manufacturing_facilities
            .iter()
            .any(|facility| matches!(facility.class.family(), 0x20..=0x21)),
        SystemSight::Hidden => false,
    }
}

/// The sector window's title colour, `0x200f0f0` (`FUN_00459e30`); the A0
/// captures show Moddell and Sesswenna titled yellow whatever their systems'
/// sides.
const SECTOR_TITLE_COLOR: egui::Color32 = crate::theme::PALETTE_YELLOW;

fn paint_resource(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    resource_id: u32,
    rect: egui::Rect,
) {
    #[cfg(test)]
    crate::fleet_window::tests::PAINTED.with(|painted| {
        painted.borrow_mut().push((resource_id, rect.min));
    });
    let Some(texture_id) = cache
        .get(ctx, DllSource::Strategy, resource_id)
        .map(egui_macroquad::egui::TextureHandle::id)
    else {
        return;
    };
    painter.image(
        texture_id,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

/// A system's three status tracks under its planet (`FUN_00459e30`), as
/// `FUN_0045b770` fills them.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StatusTrackValues {
    /// Energy slots (`+0x5c`) and those in use (`+0x60`): one per facility
    /// (manual p. 33, Fig. 2.18).
    energy: (u32, u32),
    /// Raw material slots (`+0x64`) and those mined (`+0x68`).
    raw_materials: (u32, u32),
    /// The player's popular support out of 100 (`FUN_00507270`).
    support: u32,
}

impl StatusTrackValues {
    /// The tracks the player sees: the live system's, or its frozen copy's
    /// (`FUN_0045b770` reads the galaxy view's system, the player's side's
    /// copy: `FUN_004f2d10`, `FUN_004f9a60`).
    fn shown(
        world: &GameWorld,
        fog: &FogState,
        system: &rebellion_core::world::System,
        key: SystemKey,
        player: Faction,
    ) -> Self {
        match crate::viewed_contents::system_sight(world, fog, player, key) {
            crate::viewed_contents::SystemSight::Frozen(view) => Self::of_copy(view, player),
            _ => Self::of(world, system, player),
        }
    }

    /// [`Self::of`] from a side's copy of the system.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Support is a clamped fraction of 100 and facility counts are small."
    )]
    fn of_copy(view: &rebellion_core::object_views::SystemView, player: Faction) -> Self {
        let mines = view
            .production_facilities
            .iter()
            .filter(|facility| facility.is_mine)
            .count() as u32;
        let facilities = view.defense_facilities.len()
            + view.manufacturing_facilities.len()
            + view.production_facilities.len();
        let support = if player == Faction::Alliance {
            view.popularity_alliance
        } else {
            view.popularity_empire
        };
        Self {
            energy: (u32::from(view.total_energy), facilities as u32),
            raw_materials: (u32::from(view.raw_materials), mines),
            support: (support.clamp(0.0, 1.0) * 100.0).round() as u32,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Support is a clamped fraction of 100 and facility counts are small."
    )]
    fn of(world: &GameWorld, system: &rebellion_core::world::System, player: Faction) -> Self {
        let mines = system
            .production_facilities
            .iter()
            .filter(|key| {
                world
                    .production_facilities
                    .get(**key)
                    .is_some_and(|facility| facility.is_mine)
            })
            .count() as u32;
        let facilities = system.defense_facilities.len()
            + system.manufacturing_facilities.len()
            + system.production_facilities.len();
        let support = if player == Faction::Alliance {
            system.popularity_alliance
        } else {
            system.popularity_empire
        };
        Self {
            energy: (u32::from(system.total_energy), facilities as u32),
            raw_materials: (u32::from(system.raw_materials), mines),
            support: (support.clamp(0.0, 1.0) * 100.0).round() as u32,
        }
    }
}

/// The status tracks' tooltips (`FUN_00459e30`, TEXTSTRA), each with its
/// row and rectangle from the planet's top left: energy 0x1822 "Energy
/// Consumption" and raw materials 0x1823 "Raw Materials", which
/// `FUN_0045c240` / `FUN_0045c450` complete with " used/slots" (0x1824,
/// 0x1825; A0 tooltip/132 "Energy Consumption 8/10" over 8 white slots of
/// 10, 133 "Raw Materials 2/8"), each 3 pixels a slot; popular support
/// 0x1821 "Popular Support" (134) across the planet's 37 pixels.
fn track_tooltips(values: &StatusTrackValues) -> Vec<(f32, (f32, f32, f32, f32), String)> {
    let string = |id| crate::win_tooltip::string(id).unwrap_or_default();
    let (space, slash) = (string(0x1824), string(0x1825));
    let slots = |label: u16, (total, used): (u32, u32)| {
        format!("{}{space}{used}{slash}{total}", string(label))
    };
    #[expect(clippy::cast_precision_loss, reason = "Slot counts are small.")]
    let width = |total: u32| 3.0 * total as f32;
    let mut tips = vec![
        (
            39.0,
            (0.0, 39.0, width(values.energy.0), 3.0),
            slots(0x1822, values.energy),
        ),
        (
            43.0,
            (0.0, 43.0, width(values.raw_materials.0), 3.0),
            slots(0x1823, values.raw_materials),
        ),
        (
            SUPPORT_TRACK_ROW,
            (0.0, SUPPORT_TRACK_ROW, 37.0, 3.0),
            string(0x1821),
        ),
    ];
    tips.retain(|(_, (_, _, width, _), text)| *width > 0.0 && !text.is_empty());
    tips
}

/// Whether the status track `row` rows below the planet shows. The tracks
/// are created hidden (`FUN_00459e30` clears their visible bit) and shown
/// by their refresh:
/// - energy and raw materials (`FUN_0045c240`, `FUN_0045c450`) for an
///   explored system (record `+0x88` bit 1);
/// - popular support (`FUN_0045baf0`) for a populated system (`+0x88` bit
///   0) with a side (`+0x24 & 0xc0`);
/// - none for a destroyed system (`+0x50` bit 3), whose refresh hides them.
///
/// port: the viewer's record of an unexplored system holds no side, so the
/// support track also needs the system explored. The A0 Moddell capture
/// shows its unexplored planets with no track, and explored Vjun with all
/// three.
fn track_shown(system: &rebellion_core::world::System, row: f32) -> bool {
    let explored = system.exploration_status == rebellion_core::dat::ExplorationStatus::Explored;
    if system.is_destroyed || !explored {
        return false;
    }
    row < SUPPORT_TRACK_ROW || system.is_populated
}

/// The support track's row below the planet's top.
const SUPPORT_TRACK_ROW: f32 = 47.0;

/// Where a planet's name starts below its top (the label item's `+0x34`,
/// set to the bitmap height plus 11 in `FUN_00459e30`).
const NAME_OFFSET: f32 = 48.0;

/// The status tracks' filled rectangles, in window pixels from a planet at
/// (`x`, `y`) (`FUN_00459e30`, `FUN_004acec0`). Each track is 3 pixels
/// tall: the first 2 below the 37 by 37 planet, each next 1 below the last.
/// Energy (white in use, blue free) and raw materials (yellow mined,
/// orange free) are resized to 3 pixels per slot (`FUN_0045c240`,
/// `FUN_0045c450`); frame 0480 of the failed multiplayer recording shows
/// each slot as a 2-pixel block after a 1-pixel gap. Popular support spans
/// the planet's 37 pixels, the player's side's color first (Alliance red
/// `0x0000ff`, Empire green `0x00ff00`, `COLORREF`).
fn status_tracks(
    x: f32,
    y: f32,
    values: &StatusTrackValues,
    player: Faction,
) -> Vec<((f32, f32, f32, f32), egui::Color32)> {
    const WHITE: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    const BLUE: egui::Color32 = egui::Color32::from_rgb(0, 0, 255);
    const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 255, 0);
    const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xf9, 0x5c, 0x0f);
    const RED: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);
    const GREEN: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);
    const TICK: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
    let mut rects = Vec::new();
    let mut slots = |row: f32, (total, used): (u32, u32), full, empty| {
        for slot in 0..total {
            let color = if slot < used { full } else { empty };
            #[expect(
                clippy::cast_precision_loss,
                reason = "Slot counts are far below f32's integer range."
            )]
            let left = x + 3.0 * slot as f32 + 1.0;
            rects.push(((left, row, 2.0, 3.0), color));
        }
        // The tick before each slot, pen 0x808080 over the fill (tick step
        // 1, FUN_0060e400; FUN_0060e3b0; Class_0065c058_vt14 = FUN_004ad210
        // draws x = slot * width / slots).
        for slot in 0..total {
            #[expect(
                clippy::cast_precision_loss,
                reason = "Slot counts are far below f32's integer range."
            )]
            let left = x + 3.0 * slot as f32;
            rects.push(((left, row, 1.0, 3.0), TICK));
        }
    };
    slots(y + 39.0, values.energy, WHITE, BLUE);
    slots(y + 43.0, values.raw_materials, YELLOW, ORANGE);
    let (own, other) = if player == Faction::Alliance {
        (RED, GREEN)
    } else {
        (GREEN, RED)
    };
    #[expect(clippy::cast_precision_loss, reason = "Support is at most 100.")]
    let filled = (37.0 * values.support.min(100) as f32 / 100.0).round();
    rects.push(((x, y + SUPPORT_TRACK_ROW, filled, 3.0), own));
    rects.push((
        (x + filled, y + SUPPORT_TRACK_ROW, 37.0 - filled, 3.0),
        other,
    ));
    rects.retain(|((_, _, width, _), _)| *width > 0.0);
    rects
}

fn paint_selection_brackets(painter: &egui::Painter, rect: egui::Rect, scale: f32) {
    let color = egui::Color32::from_rgb(255, 32, 32);
    let stroke = egui::Stroke::new(scale.max(1.0), color);
    let length = 5.0 * scale;
    for (corner, dx, dy) in [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.left_bottom(), 1.0, -1.0),
        (rect.right_bottom(), -1.0, -1.0),
    ] {
        painter.line_segment(
            [corner, egui::pos2(corner.x + dx * length, corner.y)],
            stroke,
        );
        painter.line_segment(
            [corner, egui::pos2(corner.x, corner.y + dy * length)],
            stroke,
        );
    }
}

#[cfg(test)]
#[path = "sector_window_tests.rs"]
mod tests;
