//! The original Build Selection window (`FUN_00437df0` builds it,
//! `FUN_00437f80` and `FUN_00438620` its fields and controls, `FUN_00438800`
//! handles them; `ghidra/notes/manufacturing-build-selection.md`).
//!
//! A 210 by 261 window over background 10800 that a Manufacturing window
//! band's Build opens. It lists the classes the band's yards build, shows
//! the order's two costs and its best completion and deployment times, and
//! takes the number to build. Confirm replaces what the band was building.

use egui_macroquad::egui;
use rebellion_core::build_selection::{estimate, BuildEstimate};
use rebellion_core::delivery::Delivery;
use rebellion_core::ids::SystemKey;
use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, ProductionArea};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::CockpitLayout;
use crate::manufacturing_window::product;
use crate::mission_dialog::{button, galaxy_centered_rect, paint, paint_centered};
use crate::system_window::{logical_rect, rect_contains};

pub const BUILD_SELECTION_WIDTH: f32 = 210.0;
pub const BUILD_SELECTION_HEIGHT: f32 = 261.0;

// STRATEGY bitmaps (FUN_00437df0, FUN_00438500, FUN_00438620).
const BACKGROUND: u32 = 10800;
const TITLE_ALLIANCE: u32 = 10801;
const TITLE_EMPIRE: u32 = 10802;
const CLOSE: (u32, u32) = (10108, 10109);
const LIST: (u32, u32) = (10606, 10607);
const CONFIRM: (u32, u32) = (10594, 10595);
/// Confirm's disabled art (`FUN_00438620`: `FUN_00603150(2, 0x2d64)` and
/// `FUN_00603150(4, 0x2d64)`).
const CONFIRM_DISABLED: u32 = 11620;
const CANCEL: (u32, u32) = (10596, 10597);
const ENCYCLOPEDIA: (u32, u32) = (10592, 10593);
const MORE: (u32, u32) = (10610, 10611);
const FEWER: (u32, u32) = (10612, 10613);

// TEXTSTRA strings (FUN_00438500, FUN_00438dd0, FUN_00438f30).
/// 14352, the title.
const TITLE: &str = "Build Selection";
/// 14353.
const NUMBER_TO_BUILD: &str = "Number to build:";
/// 14354.
const BEST_COMPLETION: &str = "Best Time To Completion:";
/// 14355.
const BEST_DEPLOYMENT: &str = "Best Time To Deployment:";
/// 14357, after a time's number.
const DAYS: &str = " Days";
/// 14358, a value of an order that cannot be built.
const NOT_AVAILABLE: &str = "n/a";

// Geometry (FUN_00437f80, FUN_00438500, FUN_00438620), in window pixels.
/// The selected class, 195 by 63 at (6, 22) (list `FUN_0060bed0`).
const ITEM_BOX: (f32, f32, f32, f32) = (6.0, 22.0, 195.0, 63.0);
/// Where an item's name line starts, from the item's top. hyp: A0 inks the
/// name in the box's rows 53..62 whatever the portrait (build-selection/10,
/// pending-facility/32), so the list draws it at a fixed place.
const NAME_TOP: f32 = 49.0;
/// Refined material (`+0x134`) and maintenance (`+0x138`), 64 by 23.
const REFINED_FIELD: (f32, f32) = (36.0, 110.0);
const MAINTENANCE_FIELD: (f32, f32) = (138.0, 110.0);
/// Best completion (`+0x12c`) and deployment (`+0x130`), 60 by 15.
const COMPLETION_FIELD: (f32, f32) = (140.0, 145.0);
const DEPLOYMENT_FIELD: (f32, f32) = (140.0, 165.0);
/// The number to build, an edit of 45 by 17 (`+0x128`).
const QUANTITY_FIELD: (f32, f32, f32, f32) = (141.0, 196.0, 45.0, 17.0);

/// The labels' color, `0x2f0fbff` (`FUN_00438500`).
const LABEL: egui::Color32 = egui::Color32::from_rgb(0xff, 0xfb, 0xf0);

/// The most units one order builds (`FUN_00438c30`, `FUN_00438c60`).
pub const MAX_QUANTITY: u32 = 255;

/// The number edit's text limit, `FUN_006050c0(0x19)` from `FUN_00437f80`.
const NUMBER_LIMIT: usize = 0x19;

/// The number an edit's text orders (`FUN_00438d30`): its leading digits as
/// `atoi` reads them (`FUN_00619560`: blanks, a sign, then digits), held
/// to 1 through 255. A minus sign makes any number less than 1, so it
/// holds to 1 as no digits do. port: a number past `i64` saturates instead
/// of wrapping.
fn number_of(text: &str) -> u32 {
    let text = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let digits = text.strip_prefix('+').unwrap_or(text);
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0_i64, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(i64::from(digit - b'0'))
        });
    u32::try_from(value.clamp(1, i64::from(MAX_QUANTITY))).unwrap_or(1)
}

/// The class list's box: under the 195 by 63 item at (6, 22).
const CLASS_LIST: crate::selection_list::DropDown = crate::selection_list::DropDown {
    x: 6.0,
    top: 85.0,
    width: 195.0,
    item_height: 70.0,
    visible: 2,
};
/// The shown class's name in the list: the item's `+0x60`
/// (`FUN_00437880`), gray (A0 manufacturing/24).
const LIST_SHOWN_TEXT: egui::Color32 = egui::Color32::from_rgb(0x80, 0x80, 0x80);

/// One open window: the band it orders for and its controls' state.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildSelection {
    system: SystemKey,
    area: ProductionArea,
    is_alliance: bool,
    /// The classes `FUN_0052e580` listed when the window was built.
    kinds: Vec<BuildableKind>,
    selected: usize,
    /// The number edit's text (`+0x128`, a `CoolStringField`): what the
    /// player typed, which the window reads as a number when it needs one
    /// (`FUN_00438d30`).
    number: String,
    /// The class list is open (`CoolSelectionBoxClass +0xe8`).
    list_open: bool,
    /// The class list's first visible pixel and the held press on its bar
    /// (`selection_list`).
    list_scroll: f32,
    list_hold: Option<crate::list_scroll_bar::Hold>,
    /// The agent opened it on its Build order (`FUN_0049e6b0` phase 3,
    /// `agent+0x180`).
    agent: bool,
}

impl BuildSelection {
    #[must_use]
    pub fn area(&self) -> ProductionArea {
        self.area
    }

    #[must_use]
    pub fn kind(&self) -> BuildableKind {
        self.kinds[self.selected]
    }

    #[must_use]
    pub fn quantity(&self) -> u32 {
        number_of(&self.number)
    }

    /// The number edit's text as drawn.
    #[must_use]
    pub fn number_text(&self) -> &str {
        &self.number
    }

    /// The shown class's place in the list.
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    #[must_use]
    pub fn list_open(&self) -> bool {
        self.list_open
    }

    /// Choose a listed class; an index past the list is ignored.
    pub fn select(&mut self, index: usize) {
        if index < self.kinds.len() {
            self.selected = index;
        }
    }

    /// Quantity up (`0x69`, keypad `+`, `FUN_00438c30`) or down (`0x6a`,
    /// keypad `-`, `FUN_00438c60`): the edit's number, held to 1 through
    /// 255, steps once while within them and is written back
    /// (`FUN_00438c90`). At a limit the edit keeps its text.
    pub fn step_quantity(&mut self, up: bool) {
        let number = self.quantity();
        let stepped = if up {
            (number < MAX_QUANTITY).then_some(number + 1)
        } else {
            (number > 1).then_some(number - 1)
        };
        if let Some(stepped) = stepped {
            self.number = stepped.to_string();
        }
    }

    /// A character typed into the number edit (`Class_0066e038_vt14`,
    /// `WM_CHAR`): any printable one (`FUN_0061a050`, mask `0x157`) goes in
    /// at the caret while the text is under its 25-character limit. The
    /// window opens with "1" and the caret after it (`FUN_00604f90`, caret
    /// -1), and no selection, so a digit typed at once follows the 1.
    /// port: the caret stays at the end; the arrows, Home, End, Delete,
    /// Insert's overtype and the mouse's selection are not ported.
    pub fn type_char(&mut self, typed: char) {
        if (typed.is_ascii_graphic() || typed == ' ') && self.number.len() < NUMBER_LIMIT {
            self.number.push(typed);
        }
    }

    /// Backspace in the number edit: the character before the caret goes,
    /// down to an empty edit, which orders 1 (`FUN_00438d30`).
    pub fn erase_digit(&mut self) {
        self.number.pop();
    }
}

/// Whether a Build Selection window is open, and the class each production
/// area last confirmed.
#[derive(Debug, Clone, Default)]
pub struct BuildSelectionState {
    dialog: Option<BuildSelection>,
    /// `FUN_00437f80` reopens on the class last built by the same kind of
    /// manager (`DAT_006b289c` facilities, `DAT_006b28a0` troops,
    /// `DAT_006b28a4` ships; set by `FUN_00438980`).
    remembered: [Option<BuildableKind>; 3],
}

const fn area_index(area: ProductionArea) -> usize {
    match area {
        ProductionArea::Shipyard => 0,
        ProductionArea::TrainingFacility => 1,
        ProductionArea::ConstructionYard => 2,
    }
}

impl BuildSelectionState {
    /// The state for a new or loaded game: no window open, the remembered
    /// classes kept. `DAT_006b289c`, `DAT_006b28a0` and `DAT_006b28a4` are
    /// globals of the executable, so they outlive a game (A0
    /// build-selection-empire2/13: a new Empire game's first Build on a
    /// facilities band opened on the Orbital Shipyard confirmed in the
    /// previous game).
    #[must_use]
    pub fn for_next_game(&self) -> Self {
        Self {
            dialog: None,
            remembered: self.remembered,
        }
    }

    /// Open the window for a band with the classes its manager lists. With
    /// none, nothing opens. The quantity starts at 1 (`FUN_00437f80`).
    pub fn open(
        &mut self,
        system: SystemKey,
        area: ProductionArea,
        is_alliance: bool,
        kinds: Vec<BuildableKind>,
    ) -> bool {
        if kinds.is_empty() {
            return false;
        }
        let selected = self.remembered[area_index(area)]
            .and_then(|kind| kinds.iter().position(|&listed| listed == kind))
            .unwrap_or(0);
        self.dialog = Some(BuildSelection {
            system,
            area,
            is_alliance,
            kinds,
            selected,
            number: "1".to_owned(),
            list_open: false,
            list_scroll: 0.0,
            list_hold: None,
            agent: false,
        });
        true
    }

    /// The window on the agent's Build order (`FUN_0049ec40` →
    /// `FUN_0041d640`): the same window, whose confirmed order the agent
    /// acknowledges ([`BuildSelectionAction::Confirm`]'s `agent`).
    pub fn open_for_agent(
        &mut self,
        system: SystemKey,
        area: ProductionArea,
        is_alliance: bool,
        kinds: Vec<BuildableKind>,
    ) -> bool {
        let opened = self.open(system, area, is_alliance, kinds);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.agent = true;
        }
        opened
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.dialog.is_some()
    }

    #[must_use]
    pub fn dialog(&self) -> Option<&BuildSelection> {
        self.dialog.as_ref()
    }

    pub fn dialog_mut(&mut self) -> Option<&mut BuildSelection> {
        self.dialog.as_mut()
    }

    /// Close and Cancel discard the order (`FUN_00438800`, 100 and 102).
    pub fn close(&mut self) {
        self.dialog = None;
    }

    /// Whether `point` falls on the open window, so the galaxy map under it
    /// takes no input.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.dialog.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    /// Confirm (`0x65`, `FUN_00438980`): remember the class for its kind of
    /// manager and submit the order; the window closes.
    pub fn confirm(&mut self) -> Option<BuildSelectionAction> {
        let dialog = self.dialog.take()?;
        let kind = dialog.kind();
        self.remembered[area_index(dialog.area)] = Some(kind);
        Some(BuildSelectionAction::Confirm {
            system: dialog.system,
            area: dialog.area,
            kind,
            count: dialog.quantity(),
            agent: dialog.agent,
        })
    }
}

/// What the window asks of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildSelectionAction {
    /// Build `count` units of `kind` at the band (`FUN_0041ce20`).
    Confirm {
        system: SystemKey,
        area: ProductionArea,
        kind: BuildableKind,
        count: u32,
        /// The agent's order (`FUN_0043a2a0`: `agent+0x180`).
        agent: bool,
    },
    /// Encyclopedia (`0x67`, `FUN_0041d6b0`). port: it opens the
    /// Encyclopedia; the selected class's entry is not bound yet.
    Encyclopedia,
}

/// The classes in the dialog's list order. `FUN_00437880` inserts each
/// item into a sorted list (`FUN_005f59f0`, the list's compare), and A0
/// shows them by name: a new Alliance game's construction yard lists
/// Construction Yard, GenCore Level I, KDY-150, LNR Series I, Mine, Orbital
/// Shipyard, Refinery, Training Facility (build-selection3/, 2026-11-05).
/// hyp: the compare is the names' byte order; the compare function is
/// not read. The AI keeps `listed_classes`' id order.
#[must_use]
pub fn in_list_order(world: &GameWorld, mut kinds: Vec<BuildableKind>) -> Vec<BuildableKind> {
    kinds.sort_by_cached_key(|kind| product(world, *kind).0);
    kinds
}

/// The window's screen rectangle. hyp: placed in the galaxy view like the
/// mission dialog, centered.
#[must_use]
pub fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, BUILD_SELECTION_WIDTH, BUILD_SELECTION_HEIGHT)
}

/// A cost as `FUN_00438dd0` writes it: capped at 9999, or "n/a".
#[must_use]
pub fn cost_text(value: u32, valid: bool) -> String {
    if valid {
        value.min(9999).to_string()
    } else {
        NOT_AVAILABLE.to_owned()
    }
}

/// A time as `FUN_00438f30` writes it: capped at 9999 with " Days", or
/// "n/a".
#[must_use]
pub fn days_text(value: u32, valid: bool) -> String {
    if valid {
        format!("{}{DAYS}", value.min(9999))
    } else {
        NOT_AVAILABLE.to_owned()
    }
}

/// Draw the open window, if any, and report what the player asked for.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_build_selection(
    ctx: &egui::Context,
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[Delivery],
    state: &mut BuildSelectionState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    unallocated_maintenance: i32,
) -> Option<BuildSelectionAction> {
    let dialog = state.dialog.as_mut()?;
    let scale = layout.scale;
    let window = window_rect(layout);
    let estimate: BuildEstimate = estimate(
        world,
        manufacturing,
        en_route,
        dialog.system,
        dialog.area,
        dialog.kind(),
        dialog.quantity(),
        dialog.is_alliance,
        unallocated_maintenance,
    );
    let valid = estimate.completion.is_some();
    let mut action = None;
    let mut close = false;
    let mut confirm = false;

    // FUN_00438b60: Enter confirms, Escape closes, keypad + and - step the
    // number; the number edit has the focus for digits.
    ctx.input(|input| {
        for event in &input.events {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = event
            {
                match key {
                    egui::Key::Enter if valid => confirm = true,
                    egui::Key::Escape => close = true,
                    egui::Key::Plus | egui::Key::Equals => dialog.step_quantity(true),
                    egui::Key::Minus => dialog.step_quantity(false),
                    egui::Key::Backspace => dialog.erase_digit(),
                    _ => {}
                }
            }
            // port: egui does not tell the keypad's + and - from the main
            // keys', so both step the number and neither is typed.
            if let egui::Event::Text(text) = event {
                for typed in text.chars().filter(|c| !matches!(c, '+' | '-' | '=')) {
                    dialog.type_char(typed);
                }
            }
        }
    });

    egui::Area::new(egui::Id::new("original-build-selection"))
        .fixed_pos(window.min)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(window.size(), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(frame);
            let at = |x: f32, y: f32, w: f32, h: f32| logical_rect(frame, scale, x, y, w, h);
            // FUN_00438500 / FUN_00437f80: the title in font 5, "Number to
            // build", the costs and the number edit in font 4, the time
            // labels, the times and the class list in font 10.
            use crate::theme::{paint_gdi_text, GdiAlign, GdiVAlign};

            paint(&painter, ctx, cache, BACKGROUND, frame, scale);
            // FUN_00438500: the side's 240-pixel strip at (2, 2), which the
            // window clips, then the labels.
            let strip = if dialog.is_alliance {
                TITLE_ALLIANCE
            } else {
                TITLE_EMPIRE
            };
            paint(
                &painter,
                ctx,
                cache,
                strip,
                at(2.0, 2.0, 208.0, 17.0),
                scale,
            );
            // Format 0x21: centred, from the box's top (no DT_VCENTER).
            paint_gdi_text(
                &painter,
                at(2.0, 2.0, 202.0, 14.0),
                scale,
                5,
                TITLE,
                egui::Color32::BLACK,
                (GdiAlign::Center, GdiVAlign::Top),
            );
            paint_gdi_text(
                &painter,
                at(20.0, 196.0, 118.0, 15.0),
                scale,
                4,
                NUMBER_TO_BUILD,
                LABEL,
                (GdiAlign::Right, GdiVAlign::Top),
            );
            for (label, y) in [(BEST_COMPLETION, 145.0), (BEST_DEPLOYMENT, 165.0)] {
                paint_gdi_text(
                    &painter,
                    at(10.0, y, 125.0, 15.0),
                    scale,
                    10,
                    label,
                    LABEL,
                    (GdiAlign::Left, GdiVAlign::VCenter),
                );
            }

            // The selected class: its portrait centered in the box, its name
            // near the foot (FUN_00437880's 195 by 63 item).
            let (x, y, w, h) = ITEM_BOX;
            paint_item(
                &painter,
                ctx,
                cache,
                world,
                dialog.kind(),
                at(x, y, w, h),
                scale,
            );

            // FUN_00438dd0 / FUN_00438f30: the values, white.
            for ((x, y), text) in [
                (REFINED_FIELD, cost_text(estimate.refined_material, valid)),
                (MAINTENANCE_FIELD, cost_text(estimate.maintenance, valid)),
            ] {
                paint_gdi_text(
                    &painter,
                    at(x, y, 64.0, 23.0),
                    scale,
                    4,
                    &text,
                    egui::Color32::WHITE,
                    (GdiAlign::Center, GdiVAlign::VCenter),
                );
            }
            for ((x, y), text) in [
                (
                    COMPLETION_FIELD,
                    days_text(estimate.completion.unwrap_or(0), valid),
                ),
                (DEPLOYMENT_FIELD, days_text(estimate.deployment, valid)),
            ] {
                // hyp: the field draws with DrawText, which clips to its
                // rect unless DT_NOCLIP, so a glyph's edge stays inside it.
                let field = at(x, y, 60.0, 15.0);
                paint_gdi_text(
                    &painter.with_clip_rect(field),
                    field,
                    scale,
                    10,
                    &text,
                    egui::Color32::WHITE,
                    (GdiAlign::Right, GdiVAlign::VCenter),
                );
            }
            let (x, y, w, h) = QUANTITY_FIELD;
            paint_gdi_text(
                &painter,
                at(x, y, w, h),
                scale,
                4,
                &dialog.number,
                egui::Color32::WHITE,
                (GdiAlign::Left, GdiVAlign::VCenter),
            );

            // FUN_00437f80 / FUN_00438620's tips (TEXTSTRA, module 2); the
            // close box's comes from FUN_00606e10(0x277c, 0x277d, 100,
            // 0x1903), A0 build-selection/tip-close. A disabled Done shows
            // none (A0 build-selection/tip-done).
            for (rect, id, string, enabled) in [
                ((193.0, 3.0, 14.0, 14.0), "close", 0x1903, true),
                (QUANTITY_FIELD, "quantity", 0x190c, true),
                ((79.0, 90.0, 65.0, 18.0), "list", 0x1906, true),
                ((189.0, 196.0, 13.0, 8.0), "more", 0x1907, true),
                ((189.0, 205.0, 13.0, 8.0), "fewer", 0x1908, true),
                ((5.0, 224.0, 66.0, 33.0), "encyclopedia", 0x190b, true),
                ((73.0, 224.0, 66.0, 33.0), "confirm", 0x1909, valid),
                ((141.0, 224.0, 66.0, 33.0), "cancel", 0x190a, true),
            ] {
                let (x, y, w, h) = rect;
                crate::win_tooltip::window_tool(ui, at(x, y, w, h), id, string, enabled);
            }
            if button(
                ui,
                cache,
                at(193.0, 3.0, 14.0, 14.0),
                "close",
                CLOSE,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(79.0, 90.0, 65.0, 18.0),
                "list",
                LIST,
                false,
                scale,
            ) {
                dialog.list_open = !dialog.list_open;
                // The list opens scrolled only as far as the shown class
                // needs (A0 build-selection4/08).
                dialog.list_scroll = CLASS_LIST.scroll_to(dialog.selected);
                dialog.list_hold = None;
            }
            if button(
                ui,
                cache,
                at(189.0, 196.0, 13.0, 8.0),
                "more",
                MORE,
                false,
                scale,
            ) {
                dialog.step_quantity(true);
            }
            if button(
                ui,
                cache,
                at(189.0, 205.0, 13.0, 8.0),
                "fewer",
                FEWER,
                false,
                scale,
            ) {
                dialog.step_quantity(false);
            }
            if button(
                ui,
                cache,
                at(5.0, 224.0, 66.0, 33.0),
                "encyclopedia",
                ENCYCLOPEDIA,
                false,
                scale,
            ) {
                action = Some(BuildSelectionAction::Encyclopedia);
            }
            // FUN_00439160 disables Confirm for an order that cannot be
            // built; FUN_00438620 gave its states 2 and 4 STRATEGY 11620,
            // the grey checkmark A0 manufacturing/11 shows.
            let confirm_rect = at(73.0, 224.0, 66.0, 33.0);
            if valid {
                if button(ui, cache, confirm_rect, "confirm", CONFIRM, false, scale) {
                    confirm = true;
                }
            } else {
                paint(&painter, ctx, cache, CONFIRM_DISABLED, confirm_rect, scale);
            }
            if button(
                ui,
                cache,
                at(141.0, 224.0, 66.0, 33.0),
                "cancel",
                CANCEL,
                false,
                scale,
            ) {
                close = true;
            }

            // The class list (FUN_00437f80: FUN_0060bed0(.., 6, 0x16, 0xc3,
            // 0x3f, 2, ..), +0xa4 = 0x46): 70-pixel items, two visible,
            // under the 195-wide box (selection_list). A0 manufacturing/24:
            // each item's portrait sits as in the box, its name 2 pixels lower;
            // the shown class's name gray and the others' white. hyp: the
            // mission dialog's list colours them the other way round (A0
            // kinds/31); which item state picks the item's +0x60 gray is not
            // traced.
            if dialog.list_open {
                let kinds = dialog.kinds.clone();
                let shown = dialog.selected;
                let mut scroll = dialog.list_scroll;
                let mut hold = dialog.list_hold;
                let outcome = crate::selection_list::draw(
                    ui,
                    &painter,
                    cache,
                    frame,
                    scale,
                    ui.id().with("class-list"),
                    CLASS_LIST,
                    kinds.len(),
                    &mut scroll,
                    &mut hold,
                    |items, cache, index, rect| {
                        let (_, _, w, h) = ITEM_BOX;
                        let item = egui::Rect::from_min_size(rect.min, egui::vec2(w, h) * scale);
                        let colour = if index == shown {
                            LIST_SHOWN_TEXT
                        } else {
                            egui::Color32::WHITE
                        };
                        paint_item_with(items, ctx, cache, world, kinds[index], item, scale, colour, 2.0);
                    },
                );
                dialog.list_scroll = scroll;
                dialog.list_hold = hold;
                if let Some(row) = outcome.chosen {
                    dialog.select(row);
                }
                if outcome.closed {
                    dialog.list_open = false;
                }
            }
        });

    if confirm {
        return state.confirm();
    }
    if close {
        state.close();
    }
    action
}

/// One class: its GOKRES portrait centered in the box (`FUN_00437880`) and
/// its name near the box's foot.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit painter, cache and geometry inputs at this drawing boundary."
)]
fn paint_item(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    world: &GameWorld,
    kind: BuildableKind,
    bounds: egui::Rect,
    scale: f32,
) {
    paint_item_with(painter, ctx, cache, world, kind, bounds, scale, egui::Color32::WHITE, 0.0);
}

/// [`paint_item`] with the name's colour and a drop of `text_dy` pixels.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit painter, cache and geometry inputs at this drawing boundary."
)]
fn paint_item_with(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    world: &GameWorld,
    kind: BuildableKind,
    bounds: egui::Rect,
    scale: f32,
    colour: egui::Color32,
    text_dy: f32,
) {
    let (name, portrait) = product(world, kind);
    if let Some(id) = portrait {
        let _ = paint_centered(painter, ctx, cache, DllSource::Gokres, id, bounds, scale);
    }
    // The name in font 10, centred across the item at a fixed height,
    // over the portrait's foot: A0 build-selection/10 (Construction Yard)
    // and pending-facility/32 (Mine) ink it in the same rows.
    let top = bounds.min.y + (NAME_TOP + text_dy) * scale;
    crate::theme::paint_gdi_text(
        painter,
        egui::Rect::from_min_max(egui::pos2(bounds.min.x, top), egui::pos2(bounds.max.x, top + 16.0 * scale)),
        scale,
        10,
        &name,
        colour,
        (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_window::tests::PAINTED;
    use rebellion_core::dat::{ExplorationStatus, Faction};
    use rebellion_core::ids::DatId;
    use rebellion_core::world::{
        BuildableClass, ControlKind, ManufacturingFacilityInstance, System,
    };

    /// More unallocated maintenance than any order here costs.
    const ROOMY: i32 = 1_000;

    #[test]
    fn the_list_shows_its_classes_by_name() {
        // A0 build-selection3/ (2026-11-05): KDY-150 (0x2a…) and Mine
        // (0x2c…) stand between GenCore Level I (0x22…) and Training
        // Facility (0x29…) by name, not by id.
        let mut world = GameWorld::default();
        let mut add = |raw: u32, name: &str| {
            let class = DatId::new(raw);
            world.buildable_classes.insert(
                class,
                BuildableClass { name: name.into(), ..Default::default() },
            );
            BuildableKind::ManufacturingFacility(rebellion_core::manufacturing::FacilityBuild {
                class,
                is_alliance: true,
            })
        };
        let gencore = add(0x2200_0001, "GenCore Level I");
        let training = add(0x2900_0001, "Training Facility");
        let yard = add(0x2a00_0001, "Construction Yard");
        let kdy = add(0x2a00_0002, "KDY-150");
        let mine = add(0x2c00_0001, "Mine");
        assert_eq!(
            in_list_order(&world, vec![gencore, training, yard, kdy, mine]),
            vec![yard, gencore, kdy, mine, training]
        );
    }

    fn layout() -> CockpitLayout {
        scaled(1.0)
    }

    fn scaled(scale: f32) -> CockpitLayout {
        use crate::cockpit::CockpitViewport;
        CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 640.0 * scale,
                height: 480.0 * scale,
            },
            galaxy: CockpitViewport {
                x: 10.0 + 55.0 * scale,
                y: 20.0 + 40.0 * scale,
                width: 485.0 * scale,
                height: 350.0 * scale,
            },
            scale,
        }
    }

    /// A painted line ([`crate::theme::TestLine`]): its string, its pen's
    /// left and baseline, and its font size.
    type Text = (String, egui::Pos2, f32);

    /// A system of the Alliance's with a training facility (period 4) when
    /// `yard`, and the regiment class TROOPSD 1 (cost 8, maintenance 6).
    fn galaxy(yard: bool) -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Bortras".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        let class = |refined, maintenance, rate| BuildableClass {
            is_alliance: true,
            is_empire: true,
            refined_material_cost: refined,
            maintenance_cost: maintenance,
            processing_rate: rate,
            ..BuildableClass::default()
        };
        world
            .buildable_classes
            .insert(DatId::new(0x2900_0002), class(10, 10, 4));
        world
            .buildable_classes
            .insert(DatId::new(0x1000_0001), class(8, 6, 0));
        if yard {
            let key = world
                .manufacturing_facilities
                .insert(ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(0x2900_0002),
                    side: rebellion_core::dat::Faction::Alliance,
                    is_shipyard: false,
                });
            world.systems[system].manufacturing_facilities.push(key);
        }
        (world, system)
    }

    fn opened(system: SystemKey, is_alliance: bool) -> BuildSelectionState {
        let mut state = BuildSelectionState::default();
        assert!(state.open(
            system,
            ProductionArea::TrainingFacility,
            is_alliance,
            vec![BuildableKind::Troop(DatId::new(0x1000_0001))],
        ));
        state
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// Two idle frames, then one per entry of `frames`, the pointer at
    /// window pixel `(x, y)`; every button event lands there.
    fn drive(
        world: &GameWorld,
        state: &mut BuildSelectionState,
        point: (f32, f32),
        frames: Vec<Vec<egui::Event>>,
    ) -> Option<BuildSelectionAction> {
        drive_at(world, state, 1.0, point, frames).0
    }

    /// `drive` at a layout scale, also returning the last frame's texts.
    fn drive_at(
        world: &GameWorld,
        state: &mut BuildSelectionState,
        scale: f32,
        point: (f32, f32),
        frames: Vec<Vec<egui::Event>>,
    ) -> (Option<BuildSelectionAction>, Vec<Text>) {
        drive_with(world, state, scale, point, frames, ROOMY)
    }

    /// `drive_at` for a side with `unallocated` maintenance.
    fn drive_with(
        world: &GameWorld,
        state: &mut BuildSelectionState,
        scale: f32,
        (x, y): (f32, f32),
        frames: Vec<Vec<egui::Event>>,
        unallocated: i32,
    ) -> (Option<BuildSelectionAction>, Vec<Text>) {
        let layout = scaled(scale);
        let pos = window_rect(layout).min + egui::vec2(x, y) * scale;
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let manufacturing = ManufacturingState::new();
        let mut emitted = None;
        let mut texts = Vec::new();
        for extra in [vec![], vec![]].into_iter().chain(frames) {
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend(extra.into_iter().map(|event| match event {
                egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
                other => other,
            }));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1040.0),
                )),
                events,
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                if let Some(action) = draw_build_selection(
                    ctx,
                    world,
                    &manufacturing,
                    &[],
                    state,
                    layout,
                    &mut cache,
                    unallocated,
                ) {
                    emitted = Some(action);
                }
            });
            texts = crate::theme::test_lines(&output.shapes)
                .into_iter()
                .map(|line| (line.text, line.pen, line.size))
                .collect();
        }
        (emitted, texts)
    }

    fn click(
        world: &GameWorld,
        state: &mut BuildSelectionState,
        point: (f32, f32),
    ) -> Option<BuildSelectionAction> {
        let at = egui::Pos2::ZERO;
        drive(
            world,
            state,
            point,
            vec![vec![press(at, true)], vec![press(at, false)]],
        )
    }

    // FUN_00437df0: background 10800 at (0, 0); FUN_00438500: the side's
    // strip at (2, 2), 10801 for side 1 and 10802 for the other;
    // FUN_00438620: Confirm 10594 at (73, 224), or 11620 while disabled
    // (the galaxy's one yard is the Alliance's; A0 manufacturing/11).
    #[test]
    fn the_window_paints_its_background_side_strip_and_controls() {
        let (world, system) = galaxy(true);
        for (is_alliance, strip, confirm) in [(true, 10_801, 10_594), (false, 10_802, 11_620)] {
            PAINTED.with(|painted| painted.borrow_mut().clear());
            let mut state = opened(system, is_alliance);
            drive(&world, &mut state, (0.0, 0.0), vec![]);
            let origin = window_rect(layout()).min;
            let painted = PAINTED.with(|painted| painted.borrow().clone());
            for (id, x, y) in [
                (10_800, 0.0, 0.0),
                (strip, 2.0, 2.0),
                (10_606, 79.0, 90.0),
                (10_592, 5.0, 224.0),
                (confirm, 73.0, 224.0),
                (10_596, 141.0, 224.0),
                (10_610, 189.0, 196.0),
                (10_612, 189.0, 205.0),
            ] {
                assert!(
                    painted.contains(&(id, origin + egui::vec2(x, y))),
                    "{id} at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn the_agents_window_confirms_as_the_agents_order() {
        // FUN_0049ec40 -> FUN_0041d640 opens the same window; its confirmed
        // order is agent+0x180, which FUN_0043a2a0 acknowledges.
        let mut state = BuildSelectionState::default();
        assert!(!state.open_for_agent(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            Vec::new()
        ));
        assert!(!state.is_open());
        assert!(state.open_for_agent(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            kinds()
        ));
        assert!(matches!(
            state.confirm(),
            Some(BuildSelectionAction::Confirm { agent: true, .. })
        ));
        assert!(state.open(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            kinds()
        ));
        assert!(matches!(
            state.confirm(),
            Some(BuildSelectionAction::Confirm { agent: false, .. })
        ));
    }

    // FUN_00438800 case 0x65 confirms; FUN_00439160 disables Confirm when
    // the manager has no yard to build with.
    #[test]
    fn confirm_orders_the_build_only_when_a_yard_can_build_it() {
        let (world, system) = galaxy(true);
        let mut state = opened(system, true);
        assert_eq!(
            click(&world, &mut state, (100.0, 240.0)),
            Some(BuildSelectionAction::Confirm {
                system,
                area: ProductionArea::TrainingFacility,
                kind: BuildableKind::Troop(DatId::new(0x1000_0001)),
                count: 1,
                agent: false,
            })
        );
        assert!(!state.is_open());

        let (world, system) = galaxy(false);
        let mut state = opened(system, true);
        assert_eq!(click(&world, &mut state, (100.0, 240.0)), None);
        assert!(state.is_open());
        assert_eq!(
            drive(
                &world,
                &mut state,
                (0.0, 0.0),
                vec![vec![key(egui::Key::Enter)]]
            ),
            None
        );
        assert!(state.is_open());
    }

    // FUN_0052e970 refuses an order whose maintenance exceeds the side's
    // unallocated maintenance; FUN_00439160 then writes 14358 "n/a" in all
    // four values and disables Confirm (A0 manufacturing/11, the Empire at
    // -402).
    #[test]
    fn an_order_over_the_sides_maintenance_shows_na_and_greys_confirm() {
        let (world, system) = galaxy(true);
        let mut state = opened(system, true);
        let release = egui::Pos2::ZERO;
        PAINTED.with(|painted| painted.borrow_mut().clear());
        let (action, texts) = drive_with(
            &world,
            &mut state,
            1.0,
            (100.0, 240.0),
            vec![vec![press(release, true)], vec![press(release, false)]],
            5,
        );
        assert_eq!(action, None);
        assert!(state.is_open());
        let na = texts.iter().filter(|(text, ..)| text == "n/a").count();
        assert_eq!(na, 4, "{texts:?}");
        // FUN_00438620: the disabled Confirm draws 11620.
        let confirm = window_rect(layout()).min + egui::vec2(73.0, 224.0);
        let painted = PAINTED.with(|painted| painted.borrow().clone());
        assert!(painted.contains(&(11_620, confirm)), "{painted:?}");
        assert!(!painted.contains(&(10_594, confirm)));

        // The regiment's 6 fit in 6.
        let (action, _) = drive_with(
            &world,
            &mut state,
            1.0,
            (100.0, 240.0),
            vec![vec![press(release, true)], vec![press(release, false)]],
            6,
        );
        assert!(matches!(action, Some(BuildSelectionAction::Confirm { .. })));
    }

    // FUN_00438b60: Enter confirms, Escape closes, keypad + steps the
    // number; FUN_00438800: Cancel (0x66) discards.
    #[test]
    fn keys_and_cancel_drive_the_window() {
        let (world, system) = galaxy(true);
        let mut state = opened(system, true);
        drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Plus)]],
        );
        assert_eq!(state.dialog().unwrap().quantity(), 2);
        let digit = egui::Event::Text("4".into());
        drive(&world, &mut state, (0.0, 0.0), vec![vec![digit]]);
        assert_eq!(state.dialog().unwrap().quantity(), 24);
        drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Backspace)]],
        );
        assert_eq!(state.dialog().unwrap().quantity(), 2);
        drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Minus)]],
        );
        assert_eq!(state.dialog().unwrap().quantity(), 1);
        drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Plus)]],
        );
        let confirmed = drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Enter)]],
        );
        assert!(matches!(
            confirmed,
            Some(BuildSelectionAction::Confirm { count: 2, .. })
        ));

        let mut state = opened(system, true);
        drive(
            &world,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Escape)]],
        );
        assert!(!state.is_open());

        let mut state = opened(system, true);
        assert_eq!(click(&world, &mut state, (170.0, 240.0)), None);
        assert!(!state.is_open());
    }

    fn kinds() -> Vec<BuildableKind> {
        vec![
            BuildableKind::Troop(DatId::new(0x1000_0001)),
            BuildableKind::Troop(DatId::new(0x1000_0002)),
        ]
    }

    fn open() -> BuildSelectionState {
        let mut state = BuildSelectionState::default();
        assert!(state.open(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            kinds()
        ));
        state
    }

    #[test]
    fn a_manager_with_nothing_to_build_opens_no_window() {
        let mut state = BuildSelectionState::default();
        assert!(!state.open(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            Vec::new()
        ));
        assert!(!state.is_open());
    }

    // FUN_00438c30 / FUN_00438c60: the number to build stays in 1..=255.
    #[test]
    fn the_number_to_build_starts_at_one_and_stays_within_1_to_255() {
        let mut state = open();
        let dialog = state.dialog_mut().unwrap();
        assert_eq!(dialog.quantity(), 1);
        dialog.step_quantity(false);
        assert_eq!(dialog.quantity(), 1);
        dialog.step_quantity(true);
        assert_eq!(dialog.quantity(), 2);
        dialog.type_char('5');
        assert_eq!(dialog.quantity(), 25);
        dialog.type_char('9');
        assert_eq!(dialog.quantity(), 255);
        dialog.step_quantity(true);
        assert_eq!(dialog.quantity(), 255);
        dialog.erase_digit();
        assert_eq!(dialog.quantity(), 25);
        dialog.erase_digit();
        dialog.erase_digit();
        assert_eq!(dialog.quantity(), 1);
    }

    // The number edit is a CoolStringField (FUN_00604cf0) holding text:
    // WM_CHAR (Class_0066e038_vt14) puts a printable character at the
    // caret under the 25-character limit (FUN_006050c0(0x19)), Backspace
    // can empty it, and FUN_00438d30 reads atoi(text) held to 1..=255
    // without writing it back.
    #[test]
    fn backspace_empties_the_number_so_digits_typed_next_stand_alone() {
        let mut state = open();
        let dialog = state.dialog_mut().unwrap();
        dialog.erase_digit();
        assert_eq!(dialog.number_text(), "");
        assert_eq!(dialog.quantity(), 1);
        dialog.type_char('9');
        dialog.type_char('9');
        assert_eq!(dialog.number_text(), "99");
        assert_eq!(dialog.quantity(), 99);
    }

    #[test]
    fn a_number_over_255_stays_as_typed_and_orders_255() {
        let mut state = open();
        let dialog = state.dialog_mut().unwrap();
        dialog.type_char('0');
        dialog.type_char('0');
        assert_eq!(dialog.number_text(), "100");
        dialog.type_char('0');
        assert_eq!(dialog.number_text(), "1000");
        assert_eq!(dialog.quantity(), 255);
        // Up at the limit leaves the text; down writes the stepped number.
        dialog.step_quantity(true);
        assert_eq!(dialog.number_text(), "1000");
        dialog.step_quantity(false);
        assert_eq!(dialog.number_text(), "254");
        // Letters go in too; atoi stops at them.
        dialog.type_char('x');
        assert_eq!((dialog.number_text(), dialog.quantity()), ("254x", 254));
        for _ in 0..40 {
            dialog.type_char('7');
        }
        assert_eq!(dialog.number_text().len(), 25);
        let action = state.confirm();
        assert!(matches!(
            action,
            Some(BuildSelectionAction::Confirm { count: 254, .. })
        ));
    }

    // FUN_00438d30 reads the text with atoi (FUN_00619560: blanks, a
    // sign, digits) and holds it to 1..=255; FUN_00438c60 steps down only
    // above 1, so at 1 the edit keeps its text.
    #[test]
    fn the_number_reads_its_text_as_atoi_does_and_down_at_one_keeps_it() {
        assert_eq!(number_of(" 12x"), 12);
        assert_eq!(number_of("+7"), 7);
        assert_eq!(number_of("-5"), 1);
        assert_eq!(number_of("-300"), 1);
        assert_eq!(number_of("x5"), 1);
        assert_eq!(number_of("99999999999999999999999"), 255);
        let mut state = open();
        let dialog = state.dialog_mut().unwrap();
        dialog.step_quantity(false);
        assert_eq!(dialog.number_text(), "1");
        // A blank goes in like any printable character.
        dialog.erase_digit();
        dialog.type_char(' ');
        dialog.type_char('8');
        assert_eq!((dialog.number_text(), dialog.quantity()), (" 8", 8));
    }

    // FUN_00438980 remembers the confirmed class for its kind of manager;
    // FUN_00437f80 reopens on it.
    #[test]
    fn confirm_submits_the_order_and_the_next_window_reopens_on_its_class() {
        let mut state = open();
        let dialog = state.dialog_mut().unwrap();
        dialog.select(1);
        dialog.select(2);
        dialog.select(9);
        dialog.step_quantity(true);
        let action = state.confirm();
        assert_eq!(
            action,
            Some(BuildSelectionAction::Confirm {
                system: SystemKey::default(),
                area: ProductionArea::TrainingFacility,
                kind: kinds()[1],
                count: 2,
                agent: false,
            })
        );
        assert!(!state.is_open());
        assert!(state.open(
            SystemKey::default(),
            ProductionArea::TrainingFacility,
            true,
            kinds()
        ));
        assert_eq!(state.dialog().unwrap().kind(), kinds()[1]);
        // Another kind of manager keeps its own.
        assert!(state.open(
            SystemKey::default(),
            ProductionArea::Shipyard,
            true,
            kinds()
        ));
        assert_eq!(state.dialog().unwrap().kind(), kinds()[0]);
    }

    // The drop-down (command 0x6c) opens under the class box; the wheel
    // walks the list and a click closes it.
    #[test]
    fn the_class_list_names_the_shown_class_gray_and_the_others_white() {
        // A0 manufacturing/24: "Construction Yard", the shown class, gray
        // (0x808080, the item's +0x60) in the list, "GenCore Level I"
        // white; each name 2 pixels lower than the box's; at any scale.
        for scale in [1.0, 2.0] {
            let (world, system) = galaxy(true);
            let mut state = BuildSelectionState::default();
            assert!(state.open(system, ProductionArea::TrainingFacility, true, kinds()));
            state.dialog.as_mut().unwrap().list_open = true;
            let layout = scaled(scale);
            let ctx = egui::Context::default();
            crate::theme::load_fonts(&ctx);
            let mut cache = BmpCache::new();
            let manufacturing = ManufacturingState::new();
            let mut output = None;
            for _ in 0..2 {
                output = Some(ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1400.0, 1040.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = draw_build_selection(
                            ctx,
                            &world,
                            &manufacturing,
                            &[],
                            &mut state,
                            layout,
                            &mut cache,
                            ROOMY,
                        );
                    },
                ));
            }
            let origin = window_rect(layout).min;
            let lines = crate::theme::test_lines(&output.unwrap().shapes);
            let named = |name: &str| -> Vec<(f32, egui::Color32)> {
                lines
                    .iter()
                    .filter(|line| line.text == name)
                    .map(|line| ((line.pen.y - origin.y) / scale, line.color))
                    .collect()
            };
            let first = named("Alliance Fleet Regiment");
            let second = named("Alliance Army Regiment");
            // The box's line, then the list's: gray and 2 lower than the
            // box's relative to its item (box at 22, list item at 87).
            assert_eq!(first.len(), 2, "{lines:?}");
            assert_eq!(first[0].1, egui::Color32::WHITE);
            assert_eq!(first[1].1, LIST_SHOWN_TEXT);
            assert!((first[1].0 - 87.0 - (first[0].0 - 22.0 + 2.0)).abs() < 0.01, "{first:?}");
            assert_eq!(second.len(), 1, "{lines:?}");
            assert_eq!(second[0].1, egui::Color32::WHITE);
            assert!((second[0].0 - first[1].0 - 70.0).abs() < 0.01, "{second:?} {first:?}");
        }
    }


    #[test]
    fn the_last_confirmed_class_outlives_the_game() {
        // DAT_006b289c / DAT_006b28a0 / DAT_006b28a4 are globals: A0
        // build-selection-empire2/13 opened a new game's facilities Build
        // on the Orbital Shipyard confirmed in the game before.
        let (_, system) = galaxy(true);
        let regiments = kinds();
        let mut state = BuildSelectionState::default();
        assert!(state.open(system, ProductionArea::TrainingFacility, true, regiments.clone()));
        state.dialog_mut().unwrap().select(1);
        assert!(state.confirm().is_some());
        let mut next = state.for_next_game();
        assert!(!next.is_open());
        assert!(next.open(system, ProductionArea::TrainingFacility, true, regiments.clone()));
        assert_eq!(next.dialog().unwrap().kind(), regiments[1]);
        // A fresh state, as the port's new game made before, opens on the
        // first class.
        let mut fresh = BuildSelectionState::default();
        assert!(fresh.open(system, ProductionArea::TrainingFacility, true, regiments.clone()));
        assert_eq!(fresh.dialog().unwrap().kind(), regiments[0]);
    }

    #[test]
    fn the_class_list_opens_scrolls_on_its_bar_and_a_click_chooses_a_class() {
        // FUN_00437f80: FUN_0060bed0(.., 6, 0x16, 0xc3, 0x3f, 2, ..), items
        // 70 high (+0xa4 = 0x46), two visible: the list at (8, 87), 191 by
        // 140, under the box. The same CoolSelectionBoxClass as the mission
        // dialog's: A0 kinds/02 shows its wheel inert, manufacturing/24 its
        // items' names 53 under each item's top.
        let (world, system) = galaxy(true);
        let mut state = BuildSelectionState::default();
        let regiments = kinds();
        assert!(state.open(
            system,
            ProductionArea::TrainingFacility,
            true,
            regiments.clone()
        ));
        let at = egui::Pos2::ZERO;
        let click = |state: &mut BuildSelectionState, point| {
            drive(
                &world,
                state,
                point,
                vec![vec![press(at, true)], vec![press(at, false)]],
            );
        };
        click(&mut state, (111.0, 99.0));
        assert!(state.dialog().unwrap().list_open());
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            modifiers: egui::Modifiers::default(),
        };
        drive(&world, &mut state, (100.0, 147.0), vec![vec![wheel]]);
        let dialog = state.dialog().unwrap();
        assert_eq!((dialog.kind(), dialog.list_scroll), (regiments[0], 0.0));
        assert_eq!(dialog.selected_index(), 0);
        assert!(dialog.list_open());

        // The second item's row (87 + 70 .. 87 + 140) chooses it and closes
        // the list.
        click(&mut state, (100.0, 87.0 + 100.0));
        let dialog = state.dialog().unwrap();
        assert!(!dialog.list_open());
        assert_eq!(dialog.kind(), regiments[1]);
        assert_eq!(dialog.selected_index(), 1);
        assert_eq!(CLASS_LIST.height(), 144.0);
        // Reopened, the list scrolls from its top only as far as the shown
        // class needs to be seen, as the last of the two items: A0
        // build-selection4 (Kessel, a new Alliance game) opened it on
        // Dreadnaught and Escort Carrier with either of them shown, and on
        // KDY-150 and LNR Series I with LNR, the fourth class, shown
        // (08-list-opened-with-lnr-shown).
        click(&mut state, (111.0, 99.0));
        assert_eq!(state.dialog().unwrap().list_scroll, 0.0);
        assert_eq!(CLASS_LIST.scroll_to(0), 0.0);
        assert_eq!(CLASS_LIST.scroll_to(1), 0.0);
        assert_eq!(CLASS_LIST.scroll_to(3), 140.0);
        assert_eq!(CLASS_LIST.list(), (8.0, 87.0, 191.0, 140.0));
    }

    #[test]
    fn the_window_blocks_the_map_only_while_open() {
        let (_, system) = galaxy(true);
        let mut state = BuildSelectionState::default();
        let inside = window_rect(layout()).center();
        let point = (inside.x, inside.y);
        assert!(!state.contains_screen_point(layout(), point));
        state = opened(system, true);
        assert!(state.contains_screen_point(layout(), point));
        assert!(!state.contains_screen_point(layout(), (0.0, 0.0)));
    }

    // FUN_00438500 fonts 5, 4 and 10 and FUN_00437f80's value fonts, at
    // twice the size on a doubled layout; the class's name.
    #[test]
    fn texts_scale_with_the_window_and_name_the_class() {
        let (world, system) = galaxy(true);
        let mut state = opened(system, true);
        let (_, texts) = drive_at(&world, &mut state, 2.0, (0.0, 0.0), vec![]);
        let size = |wanted: &str| {
            texts
                .iter()
                .find(|(text, _, _)| text == wanted)
                .map(|(_, _, size)| *size)
        };
        // FUN_00438500: the title in font 5, "Number to build" in font 4,
        // the time labels in font 10; FUN_00437f80: the costs in font 4,
        // the times in font 10, the class list in font 10.
        let font = |n| Some(crate::theme::gdi_font(n, 2.0).size);
        assert_eq!(size(TITLE), font(5));
        assert_eq!(size(NUMBER_TO_BUILD), font(4));
        assert_eq!(size(BEST_COMPLETION), font(10));
        assert_eq!(size("32 Days"), font(10));
        assert_eq!(size("8"), font(4));
        assert_eq!(size("6"), font(4));
        assert_eq!(size("Alliance Fleet Regiment"), font(10));
    }

    #[test]
    fn the_class_name_sits_at_a_fixed_height_in_the_box() {
        // A0 build-selection/10 (Construction Yard) and pending-facility/32
        // (Mine) ink the name in the same rows, 75..84 of the window, over
        // portraits of different extents.
        let (world, system) = galaxy(true);
        let mut state = opened(system, true);
        let (_, texts) = drive_at(&world, &mut state, 1.0, (0.0, 0.0), vec![]);
        let origin = window_rect(layout()).min;
        let (_, pen, _) = texts
            .iter()
            .find(|(text, _, _)| text == "Alliance Fleet Regiment")
            .unwrap();
        assert_eq!(
            pen.y - origin.y,
            ITEM_BOX.1 + NAME_TOP + crate::theme::original_font_metrics(10).0
        );
    }

    // FUN_00438dd0 / FUN_00438f30: 9999 at most, " Days" (14357), and "n/a"
    // (14358) for an order that cannot be built.
    #[test]
    fn values_cap_at_9999_and_show_na_when_the_order_cannot_be_built() {
        assert_eq!(cost_text(24, true), "24");
        assert_eq!(cost_text(12_000, true), "9999");
        assert_eq!(cost_text(24, false), "n/a");
        assert_eq!(days_text(32, true), "32 Days");
        assert_eq!(days_text(10_000, true), "9999 Days");
        assert_eq!(days_text(32, false), "n/a");
    }
}
